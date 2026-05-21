//! Workspace-thread prompt handling for channel inputs.

use std::sync::Arc;

use agent_client_protocol::schema as acp;

use crate::agent::AgentClientHandler;
use crate::channels::bridge_handler::ChannelBridgeHandler;
use crate::channels::plugin_host::PluginHost;
use crate::channels::types::ChannelOutput;
use crate::routing::RouteKey;
use crate::workspace::manager::{ThreadChoice, WorkspaceSwitch};
use crate::workspace::threads::runtime::ThreadRuntime;
use crate::workspace::WorkspaceThreadManager;

use super::send_system_text;

pub(crate) async fn handle_prompt(
    workspace_threads: &Arc<WorkspaceThreadManager>,
    plugin_host: &Arc<PluginHost>,
    route: RouteKey,
    mut content_blocks: Vec<acp::ContentBlock>,
) -> acp::Result<acp::PromptResponse> {
    let text = first_text(&content_blocks).unwrap_or_default();

    if let Some(command) = parse_thread_command(&text) {
        return handle_command(workspace_threads, plugin_host, &route, command).await;
    }

    if let Some(runtime) = workspace_threads
        .select_pending_thread(&route, &text)
        .await
        .map_err(internal_error)?
    {
        start_runtime_and_notify(&runtime, plugin_host, &route).await?;
        send_system_text(
            plugin_host,
            &route,
            &format!("Switched to thread {}.", runtime.state().await.thread_id),
        )
        .await;
        return Ok(acp::PromptResponse::new(acp::StopReason::EndTurn));
    }

    if content_blocks.is_empty() {
        return Ok(acp::PromptResponse::new(acp::StopReason::EndTurn));
    }

    let runtime = workspace_threads
        .resolve_route_runtime(&route)
        .await
        .map_err(internal_error)?;
    start_runtime_and_notify(&runtime, plugin_host, &route).await?;
    let handler: Arc<dyn AgentClientHandler> = Arc::new(ChannelBridgeHandler::for_thread(
        Arc::clone(plugin_host),
        route.clone(),
    ));
    runtime
        .prompt(&route, std::mem::take(&mut content_blocks), handler)
        .await
}

async fn handle_command(
    workspace_threads: &Arc<WorkspaceThreadManager>,
    plugin_host: &Arc<PluginHost>,
    route: &RouteKey,
    command: ThreadCommand,
) -> acp::Result<acp::PromptResponse> {
    match command {
        ThreadCommand::New => {
            let runtime = workspace_threads
                .create_thread_in_current_workspace(route)
                .await
                .map_err(internal_error)?;
            start_runtime_and_notify(&runtime, plugin_host, route).await?;
            send_system_text(
                plugin_host,
                route,
                &format!("Started new thread {}.", runtime.state().await.thread_id),
            )
            .await;
        }
        ThreadCommand::Close => {
            workspace_threads
                .close_route(route, Some("user closed".to_string()))
                .await
                .map_err(internal_error)?;
            send_system_text(
                plugin_host,
                route,
                "Thread closed. Use /new to start again.",
            )
            .await;
        }
        ThreadCommand::SwitchWorkspace(token) => {
            match workspace_threads
                .switch_workspace(route, &token)
                .await
                .map_err(internal_error)?
            {
                WorkspaceSwitch::Started(runtime) => {
                    start_runtime_and_notify(&runtime, plugin_host, route).await?;
                    send_system_text(
                        plugin_host,
                        route,
                        &format!(
                            "Entered workspace and started thread {}.",
                            runtime.state().await.thread_id
                        ),
                    )
                    .await;
                }
                WorkspaceSwitch::NeedsSelection { workspace, threads } => {
                    send_system_text(
                        plugin_host,
                        route,
                        &format_thread_choices(&workspace.name, &threads),
                    )
                    .await;
                }
            }
        }
        ThreadCommand::Help => {
            send_system_text(
                plugin_host,
                route,
                "Commands: /switch workspace <id|path>, /new, /close. Reply with a listed number or thread id after switching workspace.",
            )
            .await;
        }
        ThreadCommand::Unknown(command) => {
            send_system_text(plugin_host, route, &format!("Unknown command: {}", command)).await;
        }
    }
    Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
}

async fn start_runtime_and_notify(
    runtime: &Arc<ThreadRuntime>,
    plugin_host: &Arc<PluginHost>,
    route: &RouteKey,
) -> acp::Result<()> {
    let before = runtime.state().await;
    let handler: Arc<dyn AgentClientHandler> = Arc::new(ChannelBridgeHandler::for_thread(
        Arc::clone(plugin_host),
        route.clone(),
    ));
    let session_id = runtime.start(route, handler).await?;
    let after = runtime.state().await;

    if before.initialize.is_none() {
        let agent_info = after
            .initialize
            .as_ref()
            .and_then(|initialize| initialize.agent_info.as_ref());
        let agent = agent_info
            .map(|info| info.title.clone().unwrap_or_else(|| info.name.clone()))
            .unwrap_or_else(|| after.host_binding.agent_id.clone());
        let version = agent_info
            .map(|info| info.version.clone())
            .unwrap_or_default();
        plugin_host
            .send_output(ChannelOutput::AgentReady {
                route: route.clone(),
                agent,
                version,
            })
            .await;
    }

    if before.session_id.as_deref() != Some(session_id.as_str()) {
        plugin_host
            .send_output(ChannelOutput::SessionReady {
                route: route.clone(),
                session_id,
            })
            .await;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThreadCommand {
    New,
    Close,
    SwitchWorkspace(String),
    Help,
    Unknown(String),
}

fn parse_thread_command(text: &str) -> Option<ThreadCommand> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized == "/new" {
        return Some(ThreadCommand::New);
    }
    if normalized == "/close" {
        return Some(ThreadCommand::Close);
    }
    if normalized == "/help" || normalized == "/commands" {
        return Some(ThreadCommand::Help);
    }
    if let Some(token) = normalized.strip_prefix("/switch workspace ") {
        let token = token.trim();
        if !token.is_empty() {
            return Some(ThreadCommand::SwitchWorkspace(token.to_string()));
        }
    }
    Some(ThreadCommand::Unknown(normalized))
}

fn first_text(content_blocks: &[acp::ContentBlock]) -> Option<String> {
    content_blocks.iter().find_map(|block| match block {
        acp::ContentBlock::Text(text) => Some(text.text.clone()),
        _ => None,
    })
}

fn format_thread_choices(workspace_name: &str, threads: &[ThreadChoice]) -> String {
    let mut lines = vec![format!("Threads in {}:", workspace_name)];
    for (index, thread) in threads.iter().enumerate() {
        let title = thread
            .first_user_prompt
            .as_deref()
            .filter(|prompt| !prompt.trim().is_empty())
            .unwrap_or("(no first prompt yet)");
        lines.push(format!(
            "{}. {} · {} · {}",
            index + 1,
            thread.thread_id,
            thread.host_binding.agent_id,
            title
        ));
    }
    lines.push("Reply with a number or thread id.".to_string());
    lines.join("\n")
}

fn internal_error(error: anyhow::Error) -> acp::Error {
    acp::Error::new(-32603, format!("{:#}", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_new_thread_commands() {
        assert_eq!(parse_thread_command("/new"), Some(ThreadCommand::New));
        assert_eq!(parse_thread_command("/close"), Some(ThreadCommand::Close));
        assert_eq!(
            parse_thread_command("/switch   workspace   general"),
            Some(ThreadCommand::SwitchWorkspace("general".to_string()))
        );
    }
}
