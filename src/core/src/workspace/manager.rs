//! Workspace/thread orchestration.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context};
use dashmap::DashMap;

use crate::agent_state;
use crate::routing::RouteKey;

use super::registry::{WorkspaceId, WorkspaceProjection, WorkspaceRecord, GENERAL_WORKSPACE_ID};
use super::store::{WorkspaceEvent, WorkspaceEventStore};
use super::threads::attachment::{
    RouteAttachmentEvent, RouteAttachmentEventStore, RouteAttachmentProjection,
};
use super::threads::runtime::ThreadRuntime;
use super::threads::store::{
    HostBinding, ThreadEvent, ThreadEventStore, ThreadProjection, WorkspaceThread,
    WorkspaceThreadId,
};

pub struct WorkspaceThreadManager {
    workspace_store: WorkspaceEventStore,
    thread_store: ThreadEventStore,
    attachment_store: RouteAttachmentEventStore,
    runtimes: DashMap<WorkspaceThreadId, Arc<ThreadRuntime>>,
    pending_selections: DashMap<RouteKey, Vec<ThreadChoice>>,
}

impl WorkspaceThreadManager {
    pub fn new_default() -> Arc<Self> {
        Arc::new(Self {
            workspace_store: WorkspaceEventStore::new(WorkspaceEventStore::default_path()),
            thread_store: ThreadEventStore::new(ThreadEventStore::default_path()),
            attachment_store: RouteAttachmentEventStore::new(
                RouteAttachmentEventStore::default_path(),
            ),
            runtimes: DashMap::new(),
            pending_selections: DashMap::new(),
        })
    }

    pub fn with_paths(
        workspace_path: PathBuf,
        thread_path: PathBuf,
        attachment_path: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            workspace_store: WorkspaceEventStore::new(workspace_path),
            thread_store: ThreadEventStore::new(thread_path),
            attachment_store: RouteAttachmentEventStore::new(attachment_path),
            runtimes: DashMap::new(),
            pending_selections: DashMap::new(),
        })
    }

    pub async fn resolve_route_runtime(
        &self,
        route: &RouteKey,
    ) -> anyhow::Result<Arc<ThreadRuntime>> {
        if let Some(attached) = self.current_attachment(route).await? {
            return self.runtime_for_thread(&attached.thread_id).await;
        }

        let workspace = self.ensure_general_workspace().await?;
        let thread = self
            .latest_open_thread(&workspace.id)
            .await?
            .unwrap_or_else(|| self.new_thread_record(workspace.id.clone()));
        self.ensure_thread_persisted(&thread).await?;
        self.attach_route(route.clone(), workspace.id, thread.id.clone())
            .await?;
        self.runtime_from_thread(thread).await
    }

    pub async fn create_thread_for_route(
        &self,
        route: &RouteKey,
        workspace_id: WorkspaceId,
    ) -> anyhow::Result<Arc<ThreadRuntime>> {
        let workspace = self
            .workspace(&workspace_id)
            .await?
            .ok_or_else(|| anyhow!("workspace {} not found", workspace_id))?;
        let thread = self.new_thread_record(workspace.id.clone());
        self.ensure_thread_persisted(&thread).await?;
        self.attach_route(route.clone(), workspace.id, thread.id.clone())
            .await?;
        self.runtime_from_thread(thread).await
    }

    pub async fn create_thread_in_current_workspace(
        &self,
        route: &RouteKey,
    ) -> anyhow::Result<Arc<ThreadRuntime>> {
        let workspace_id = self
            .current_attachment(route)
            .await?
            .map(|attachment| attachment.workspace_id)
            .unwrap_or_else(|| WorkspaceId::general());
        let workspace_id = if self.workspace(&workspace_id).await?.is_some() {
            workspace_id
        } else {
            self.ensure_general_workspace().await?.id
        };
        self.create_thread_for_route(route, workspace_id).await
    }

    pub async fn close_route(
        &self,
        route: &RouteKey,
        reason: Option<String>,
    ) -> anyhow::Result<()> {
        let Some(attached) = self.current_attachment(route).await? else {
            return Ok(());
        };
        let runtime = self.runtime_for_thread(&attached.thread_id).await?;
        runtime
            .close(reason)
            .await
            .map_err(|error| anyhow!(error.to_string()))
    }

    pub async fn switch_workspace(
        &self,
        route: &RouteKey,
        token: &str,
    ) -> anyhow::Result<WorkspaceSwitch> {
        let workspace = self
            .resolve_workspace(token)
            .await?
            .ok_or_else(|| anyhow!("workspace '{}' not found", token))?;
        let threads = self.open_threads_for_workspace(&workspace.id).await?;
        if threads.is_empty() {
            let runtime = self.create_thread_for_route(route, workspace.id).await?;
            return Ok(WorkspaceSwitch::Started(runtime));
        }
        let choices: Vec<ThreadChoice> = threads.into_iter().map(ThreadChoice::from).collect();
        self.pending_selections
            .insert(route.clone(), choices.clone());
        Ok(WorkspaceSwitch::NeedsSelection {
            workspace,
            threads: choices,
        })
    }

    pub async fn select_pending_thread(
        &self,
        route: &RouteKey,
        text: &str,
    ) -> anyhow::Result<Option<Arc<ThreadRuntime>>> {
        let Some((_, choices)) = self.pending_selections.remove(route) else {
            return Ok(None);
        };
        let token = text.trim();
        let selected = parse_thread_choice(token, &choices);
        match selected {
            Some(thread_id) => self.attach_thread(route, &thread_id).await.map(Some),
            None => {
                self.pending_selections.insert(route.clone(), choices);
                Ok(None)
            }
        }
    }

    pub async fn attach_thread(
        &self,
        route: &RouteKey,
        thread_id: &WorkspaceThreadId,
    ) -> anyhow::Result<Arc<ThreadRuntime>> {
        let thread = self
            .thread(thread_id)
            .await?
            .ok_or_else(|| anyhow!("thread {} not found", thread_id))?;
        self.attach_route(
            route.clone(),
            thread.workspace_id.clone(),
            thread.id.clone(),
        )
        .await?;
        self.runtime_from_thread(thread).await
    }

    pub async fn current_attachment(
        &self,
        route: &RouteKey,
    ) -> anyhow::Result<Option<super::threads::attachment::RouteAttachment>> {
        Ok(self.attachment_projection().await?.get(route).cloned())
    }

    async fn attach_route(
        &self,
        route: RouteKey,
        workspace_id: WorkspaceId,
        thread_id: WorkspaceThreadId,
    ) -> anyhow::Result<()> {
        self.attachment_store
            .append(&RouteAttachmentEvent::attached(
                route,
                workspace_id,
                thread_id,
            ))
            .await
            .context("append route attachment")?;
        Ok(())
    }

    async fn ensure_general_workspace(&self) -> anyhow::Result<WorkspaceRecord> {
        let projection = self.workspace_projection().await?;
        if let Some(workspace) = projection.get(&WorkspaceId::general()) {
            return Ok(workspace.clone());
        }

        let cwd = crate::config::builtin_workspaces_dir();
        if let Some(workspace) = projection.get_by_cwd(&cwd) {
            return Ok(workspace.clone());
        }

        let event = WorkspaceEvent::registered(WorkspaceId::general(), cwd, "General", true);
        self.workspace_store
            .append(&event)
            .await
            .context("append general workspace")?;
        Ok(WorkspaceProjection::from_events(&[event])?
            .get(&WorkspaceId::general())
            .cloned()
            .expect("registered general workspace"))
    }

    async fn resolve_workspace(&self, token: &str) -> anyhow::Result<Option<WorkspaceRecord>> {
        let token = token.trim();
        if token.is_empty() {
            return Ok(None);
        }
        let projection = self.workspace_projection().await?;
        if token == GENERAL_WORKSPACE_ID {
            return Ok(projection.get(&WorkspaceId::general()).cloned());
        }
        let id = WorkspaceId::from(token);
        if let Some(workspace) = projection.get(&id) {
            return Ok(Some(workspace.clone()));
        }
        let path = PathBuf::from(token);
        Ok(projection.get_by_cwd(&path).cloned())
    }

    async fn workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> anyhow::Result<Option<WorkspaceRecord>> {
        Ok(self
            .workspace_projection()
            .await?
            .get(workspace_id)
            .cloned())
    }

    async fn thread(
        &self,
        thread_id: &WorkspaceThreadId,
    ) -> anyhow::Result<Option<WorkspaceThread>> {
        Ok(self.thread_projection().await?.get(thread_id).cloned())
    }

    async fn latest_open_thread(
        &self,
        workspace_id: &WorkspaceId,
    ) -> anyhow::Result<Option<WorkspaceThread>> {
        Ok(self
            .open_threads_for_workspace(workspace_id)
            .await?
            .into_iter()
            .max_by(|a, b| a.updated_at.cmp(&b.updated_at)))
    }

    async fn open_threads_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> anyhow::Result<Vec<WorkspaceThread>> {
        Ok(self
            .thread_projection()
            .await?
            .for_workspace(workspace_id, false)
            .cloned()
            .collect())
    }

    async fn ensure_thread_persisted(&self, thread: &WorkspaceThread) -> anyhow::Result<()> {
        if self.thread(&thread.id).await?.is_some() {
            return Ok(());
        }
        self.thread_store
            .append(&ThreadEvent::created(
                thread.id.clone(),
                thread.workspace_id.clone(),
                thread.host_binding.clone(),
            ))
            .await
            .context("append workspace thread")?;
        Ok(())
    }

    fn new_thread_record(&self, workspace_id: WorkspaceId) -> WorkspaceThread {
        let host_binding = default_host_binding();
        let event = ThreadEvent::created(WorkspaceThreadId::new(), workspace_id, host_binding);
        ThreadProjection::from_events(&[event])
            .expect("single created event should project")
            .all()
            .next()
            .cloned()
            .expect("created thread")
    }

    async fn runtime_for_thread(
        &self,
        thread_id: &WorkspaceThreadId,
    ) -> anyhow::Result<Arc<ThreadRuntime>> {
        if let Some(runtime) = self.runtimes.get(thread_id) {
            return Ok(Arc::clone(runtime.value()));
        }
        let thread = self
            .thread(thread_id)
            .await?
            .ok_or_else(|| anyhow!("thread {} not found", thread_id))?;
        self.runtime_from_thread(thread).await
    }

    async fn runtime_from_thread(
        &self,
        thread: WorkspaceThread,
    ) -> anyhow::Result<Arc<ThreadRuntime>> {
        if let Some(runtime) = self.runtimes.get(&thread.id) {
            return Ok(Arc::clone(runtime.value()));
        }
        let workspace = self
            .workspace(&thread.workspace_id)
            .await?
            .ok_or_else(|| anyhow!("workspace {} not found", thread.workspace_id))?;
        let runtime = Arc::new(ThreadRuntime::new(
            thread.clone(),
            workspace.cwd,
            self.thread_store.clone(),
        ));
        self.runtimes.insert(thread.id, Arc::clone(&runtime));
        Ok(runtime)
    }

    async fn workspace_projection(&self) -> anyhow::Result<WorkspaceProjection> {
        self.workspace_store
            .load_projection()
            .await
            .map_err(|error| anyhow!(error.to_string()))
    }

    async fn thread_projection(&self) -> anyhow::Result<ThreadProjection> {
        self.thread_store
            .load_projection()
            .await
            .map_err(|error| anyhow!(error.to_string()))
    }

    async fn attachment_projection(&self) -> anyhow::Result<RouteAttachmentProjection> {
        self.attachment_store
            .load_projection()
            .await
            .map_err(|error| anyhow!(error.to_string()))
    }
}

fn parse_thread_choice(token: &str, choices: &[ThreadChoice]) -> Option<WorkspaceThreadId> {
    if let Ok(index) = token.parse::<usize>() {
        if index > 0 {
            return choices
                .get(index - 1)
                .map(|choice| choice.thread_id.clone());
        }
    }
    choices
        .iter()
        .find(|choice| choice.thread_id.as_str() == token)
        .map(|choice| choice.thread_id.clone())
}

#[derive(Debug, Clone)]
pub struct ThreadChoice {
    pub thread_id: WorkspaceThreadId,
    pub host_binding: HostBinding,
    pub updated_at: String,
    pub first_user_prompt: Option<String>,
}

impl From<WorkspaceThread> for ThreadChoice {
    fn from(thread: WorkspaceThread) -> Self {
        Self {
            thread_id: thread.id,
            host_binding: thread.host_binding,
            updated_at: thread.updated_at,
            first_user_prompt: thread.first_user_prompt,
        }
    }
}

pub enum WorkspaceSwitch {
    Started(Arc<ThreadRuntime>),
    NeedsSelection {
        workspace: WorkspaceRecord,
        threads: Vec<ThreadChoice>,
    },
}

fn default_host_binding() -> HostBinding {
    let cfg = crate::config::ensure_loaded();
    let prefs = agent_state::read_prefs();
    let agent_id = agent_state::resolve_default_agent(&prefs, &cfg);
    let profile_id = agent_state::resolve_default_profile(&prefs, &cfg, &agent_id)
        .or(Some("direct".to_string()));
    HostBinding::new(agent_id, profile_id)
}

#[allow(dead_code)]
fn workspace_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Workspace")
        .to_string()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn temp_paths() -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("vibearound-wtm-{}", Uuid::new_v4()));
        (
            root.join("workspaces.jsonl"),
            root.join("threads.jsonl"),
            root.join("attachments.jsonl"),
        )
    }

    #[tokio::test]
    async fn route_resolves_to_stable_thread_attachment() {
        let (workspaces, threads, attachments) = temp_paths();
        let manager = WorkspaceThreadManager::with_paths(workspaces, threads, attachments);
        let route = RouteKey::new("feishu", "chat-a");

        let first = manager.resolve_route_runtime(&route).await.unwrap();
        let second = manager.resolve_route_runtime(&route).await.unwrap();

        assert_eq!(
            first.state().await.thread_id,
            second.state().await.thread_id
        );
        assert_eq!(
            manager
                .current_attachment(&route)
                .await
                .unwrap()
                .unwrap()
                .workspace_id,
            WorkspaceId::general()
        );
    }
}
