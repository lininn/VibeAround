//! Profiles — user-managed third-party API credentials + one-click launch
//! into a system Terminal.app window with the right env vars injected.
//!
//! The schema/catalog/rendering engine lives in `common::profiles` so the
//! headless core can launch IM agents with the same profile behavior.

mod launcher;
mod terminal;

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use common::agent_state;
use common::profiles::schema::{ApiTypeOverrides, ProviderSettings};
use common::profiles::{catalog, normalize_legacy_profile, runtime, schema};
use common::{config, resources};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

pub use common::profiles::{AuthMode, ProfileDef};

// ---------------------------------------------------------------------------
// View types — sanitized for the frontend.
// ---------------------------------------------------------------------------

/// List item — does NOT include credentials. Used to render the Launch tab
/// without ever shipping API keys to the webview after the initial save.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub label: String,
    pub provider: String,
    /// Provider's display label, resolved from the catalog. Falls back to
    /// the raw provider id when the catalog entry is missing — this can
    /// happen if a user keeps a profile after we ship a catalog removal.
    pub provider_label: String,
    pub provider_icon: Option<String>,
    pub auth_mode: AuthMode,
    /// API kinds this provider credential declares, e.g. `anthropic`,
    /// `openai-chat`, `gemini`. Kept as `api_types` on the wire for
    /// profile.json compatibility.
    pub api_types: Vec<String>,
    /// Concrete CLI buttons the Launch tab should render. Derived from the
    /// profile's API kinds plus each CLI target's adapter support.
    pub launch_targets: Vec<LaunchTargetSummary>,
    /// `api_type → caveat string` (subset; only the api_types that have a
    /// non-empty `compatibility_warning` in the catalog appear here). Lets
    /// the UI render a ⚠ tooltip on the affected launch button without
    /// needing the full catalog client-side.
    pub api_type_warnings: std::collections::BTreeMap<String, String>,
    /// `api_type -> model id`, sanitized for manual client setup.
    pub api_type_models: std::collections::BTreeMap<String, String>,
    /// `api_type -> catalog model options`, used by proxy route model selection.
    pub api_type_model_options: std::collections::BTreeMap<String, Vec<catalog::ModelDef>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchTargetSummary {
    pub id: String,
    pub label: String,
    pub api_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Catalog entry sent to the UI. Nested `EndpointDef` / `AuthModeDef` /
/// `FieldDef` types use snake_case keys (no rename annotation) so the
/// frontend's mustache-lite knowledge of `{{api_key}}` / `{{base_url}}`
/// stays consistent end-to-end.
#[derive(Debug, Serialize)]
pub struct CatalogEntry {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub homepage: Option<String>,
    pub endpoints: Vec<catalog::EndpointDef>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileDraft {
    pub label: String,
    pub provider: String,
    pub auth_mode: AuthMode,
    pub api_types: Vec<String>,
    #[serde(default)]
    pub credentials: BTreeMap<String, String>,
    #[serde(default)]
    pub overrides: BTreeMap<String, ApiTypeOverrides>,
    #[serde(default)]
    pub provider_settings: ProviderSettings,
}

impl ProfileDraft {
    fn into_profile(self, id: String) -> ProfileDef {
        ProfileDef {
            id,
            label: self.label,
            provider: self.provider,
            auth_mode: self.auth_mode,
            api_types: self.api_types,
            credentials: self.credentials,
            overrides: self.overrides,
            provider_settings: self.provider_settings,
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn profiles_list() -> Vec<ProfileSummary> {
    ordered_profiles()
        .into_iter()
        .map(|p| {
            let provider = catalog::get(&p.provider);
            let (label, icon) = match provider {
                Some(c) => (c.label.clone(), c.icon.clone()),
                None => (p.provider.clone(), None),
            };
            let mut api_type_warnings: std::collections::BTreeMap<String, String> =
                std::collections::BTreeMap::new();
            if let Some(c) = provider {
                for api_type in &p.api_types {
                    let endpoint_id = p
                        .overrides
                        .get(api_type)
                        .and_then(|overrides| overrides.endpoint_id.as_deref());
                    if let Some(ep) = catalog::find_endpoint(c, api_type, endpoint_id) {
                        if let Some(w) = &ep.compatibility_warning {
                            api_type_warnings.insert(api_type.clone(), w.clone());
                        }
                    }
                }
            }
            let api_type_models: std::collections::BTreeMap<String, String> = p
                .api_types
                .iter()
                .filter_map(|api_type| {
                    let endpoint = provider.and_then(|catalog| {
                        let endpoint_id = p
                            .overrides
                            .get(api_type)
                            .and_then(|overrides| overrides.endpoint_id.as_deref());
                        catalog::find_endpoint(catalog, api_type, endpoint_id)
                    });
                    let model = p
                        .overrides
                        .get(api_type)
                        .and_then(|overrides| overrides.model.as_ref())
                        .filter(|model| !model.trim().is_empty())
                        .cloned()
                        .or_else(|| {
                            endpoint
                                .and_then(|endpoint| endpoint.models.first())
                                .map(|model| model.id.clone())
                        })?;
                    Some((api_type.clone(), model))
                })
                .collect();
            let api_type_model_options = p
                .api_types
                .iter()
                .filter_map(|api_type| {
                    let endpoint = provider.and_then(|catalog| {
                        let endpoint_id = p
                            .overrides
                            .get(api_type)
                            .and_then(|overrides| overrides.endpoint_id.as_deref());
                        catalog::find_endpoint(catalog, api_type, endpoint_id)
                    });
                    let mut models = endpoint
                        .map(|endpoint| endpoint.models.clone())
                        .unwrap_or_default();
                    if let Some(model) = p
                        .overrides
                        .get(api_type)
                        .and_then(|overrides| overrides.model.as_ref())
                        .filter(|model| !model.trim().is_empty())
                    {
                        if !models.iter().any(|item| item.id == *model) {
                            models.insert(
                                0,
                                catalog::ModelDef {
                                    id: model.clone(),
                                    label: None,
                                },
                            );
                        }
                    }
                    if models.is_empty() {
                        if let Some(model) = api_type_models.get(api_type) {
                            models.push(catalog::ModelDef {
                                id: model.clone(),
                                label: None,
                            });
                        }
                    }
                    (!models.is_empty()).then_some((api_type.clone(), models))
                })
                .collect();
            let api_type_warnings_for_targets = api_type_warnings.clone();
            ProfileSummary {
                id: p.id,
                label: p.label,
                provider: p.provider,
                provider_label: label,
                provider_icon: icon,
                auth_mode: p.auth_mode,
                launch_targets: runtime::launch_targets_for_api_types(&p.api_types)
                    .into_iter()
                    .map(|(id, label, api_type)| LaunchTargetSummary {
                        id: id.to_string(),
                        label: label.to_string(),
                        api_type: api_type.to_string(),
                        warning: api_type_warnings_for_targets.get(api_type).cloned(),
                    })
                    .collect(),
                api_types: p.api_types,
                api_type_warnings,
                api_type_models,
                api_type_model_options,
            }
        })
        .collect()
}

#[tauri::command]
pub fn profiles_get(id: String) -> Result<ProfileDef, String> {
    schema::load(&id)
        .map(normalize_legacy_profile)
        .ok_or_else(|| format!("profile '{id}' not found"))
}

#[tauri::command]
pub fn profiles_upsert(app: tauri::AppHandle, profile: ProfileDef) -> Result<(), String> {
    save_profile(&app, &profile)
}

#[tauri::command]
pub fn profiles_create(app: tauri::AppHandle, draft: ProfileDraft) -> Result<ProfileDef, String> {
    let id = schema::generate_unique_id(&draft.provider).map_err(|e| e.to_string())?;
    let profile = draft.into_profile(id);
    save_profile(&app, &profile)?;
    Ok(profile)
}

fn save_profile(app: &tauri::AppHandle, profile: &ProfileDef) -> Result<(), String> {
    schema::validate(profile).map_err(|e| e.to_string())?;
    let provider = catalog::get(&profile.provider)
        .ok_or_else(|| format!("unknown provider '{}'", profile.provider))?;
    for api_type in &profile.api_types {
        let endpoint_id = profile
            .overrides
            .get(api_type)
            .and_then(|overrides| overrides.endpoint_id.as_deref());
        if catalog::find_endpoint(provider, api_type, endpoint_id).is_none() {
            let suffix = endpoint_id
                .map(|id| format!(" endpoint_id '{id}'"))
                .unwrap_or_default();
            return Err(format!(
                "provider '{}' does not support api kind '{}'{}",
                profile.provider, api_type, suffix
            ));
        }
    }
    schema::save(profile).map_err(|e| e.to_string())?;
    ensure_profile_order_contains(&profile.id)?;
    emit_launch_config_changed(app);
    Ok(())
}

#[tauri::command]
pub fn profiles_delete(app: tauri::AppHandle, id: String) -> Result<(), String> {
    schema::delete(&id).map_err(|e| e.to_string())?;
    clear_default_profile_references(&id)?;
    agent_state::remove_profile_references(&id).map_err(|e| e.to_string())?;
    terminal::remove_profile_connections(&id).map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn profiles_reorder(app: tauri::AppHandle, profile_ids: Vec<String>) -> Result<(), String> {
    let profiles: Vec<_> = schema::list()
        .into_iter()
        .map(normalize_legacy_profile)
        .collect();
    let existing_ids: HashSet<_> = profiles.iter().map(|profile| profile.id.as_str()).collect();
    let mut seen = HashSet::new();
    let mut ordered_ids = Vec::new();

    for id in profile_ids {
        let id = id.trim();
        if existing_ids.contains(id) && seen.insert(id.to_string()) {
            ordered_ids.push(id.to_string());
        }
    }

    for profile in profiles {
        if seen.insert(profile.id.clone()) {
            ordered_ids.push(profile.id);
        }
    }

    write_profile_order(&ordered_ids)?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn profiles_launch(id: String, launch_target: String) -> Result<(), String> {
    let profile = schema::load(&id)
        .map(normalize_legacy_profile)
        .ok_or_else(|| format!("profile '{id}' not found"))?;
    if !profile_can_launch_agent(&profile, &launch_target) {
        return Err(format!("profile '{id}' cannot launch '{launch_target}'"));
    }
    launcher::launch(&profile, &launch_target).map_err(|e| e.to_string())
}

/// Launch a CLI directly with no env injection — uses whatever global
/// OAuth / login session the user already has. `agent_id` is the
/// agents.json id (e.g. "claude", "codex", "gemini", "cursor", "kiro",
/// "qwen-code", "opencode").
#[tauri::command]
pub fn profiles_launch_direct(agent_id: String) -> Result<(), String> {
    launcher::launch_direct(&agent_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profiles_catalog() -> Vec<CatalogEntry> {
    catalog::all()
        .iter()
        .map(|c| CatalogEntry {
            id: c.id.clone(),
            label: c.label.clone(),
            icon: c.icon.clone(),
            homepage: c.homepage.clone(),
            endpoints: c.endpoints.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Terminal preference commands
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOption {
    pub id: String,
    pub label: String,
    pub installed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherPreferences {
    /// `id` of the currently-preferred terminal.
    pub terminal: String,
    /// Every supported terminal, with an `installed` flag the UI uses to
    /// gray out unavailable choices instead of just hiding them — keeps
    /// the dropdown stable and discoverable as users install more apps.
    pub options: Vec<TerminalOption>,
    /// Resolved cwd used for profile/direct launches.
    pub workspace: String,
    /// Suggested cwd choices surfaced in the Launch header.
    pub workspace_options: Vec<WorkspaceOption>,
    /// Canonical agent id selected in the Launch tab.
    pub selected_agent: String,
    /// Per-agent launch choices stored in `~/.vibearound/agents.json`.
    pub agent_preferences: std::collections::BTreeMap<String, AgentLaunchPreferenceSummary>,
    /// VibeAround-wide default agent for tray quick launch and IM startup.
    pub default_agent: String,
    /// Optional profile paired with the VibeAround-wide default agent.
    pub default_profile_id: Option<String>,
    /// Agent ids enabled by onboarding/settings.json.
    pub enabled_agents: Vec<String>,
    /// Back-compat alias for older UI code. New writes go to agents.json.
    pub default_profiles: std::collections::BTreeMap<String, String>,
    /// Global policy for wrapping OpenAI-compatible profile launches through
    /// VibeAround's local compatibility proxy.
    pub compatibility_proxy: terminal::CompatibilityProxyMode,
    /// Per-profile connection choices for launch targets that can run via
    /// the local API proxy.
    pub profile_connections: agent_state::ProfileConnectionPreferences,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLaunchPreferenceSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOption {
    pub path: String,
    pub label: String,
    pub detail: String,
    pub kind: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSessionSummary {
    pub agent_id: String,
    pub session_id: String,
    pub title: String,
    pub workspace: String,
    pub updated_at: u64,
    pub short_id: String,
    pub archived: bool,
}

#[tauri::command]
pub async fn launcher_get_preferences() -> Result<LauncherPreferences, String> {
    tauri::async_runtime::spawn_blocking(launcher_preferences)
        .await
        .map_err(|e| e.to_string())
}

fn launcher_preferences() -> LauncherPreferences {
    let installed_ids: std::collections::HashSet<&'static str> = terminal::detect_installed()
        .iter()
        .map(|c| c.id())
        .collect();
    let options = terminal::TerminalChoice::ALL
        .iter()
        .map(|c| TerminalOption {
            id: c.id().to_string(),
            label: c.label().to_string(),
            installed: installed_ids.contains(c.id()),
        })
        .collect();
    let cfg = config::ensure_loaded();
    let agent_prefs = agent_state::read_prefs();
    let selected_agent = agent_state::resolve_selected_agent(&agent_prefs, &cfg);
    let default_agent = agent_state::resolve_default_agent(&agent_prefs, &cfg);
    let default_profile_id =
        agent_state::resolve_default_profile(&agent_prefs, &cfg, &default_agent);
    let workspace = resolve_agent_workspace_preference(&selected_agent, &agent_prefs)
        .unwrap_or_else(|_| terminal::launch_home_dir().unwrap_or_else(|_| config::data_dir()))
        .to_string_lossy()
        .to_string();
    let agent_preferences = summarize_agent_preferences(&agent_prefs, &cfg);
    let default_profiles = agent_preferences
        .iter()
        .filter_map(|(agent_id, preference)| {
            preference
                .profile_id
                .as_ref()
                .map(|profile_id| (agent_id.clone(), profile_id.clone()))
        })
        .collect();
    LauncherPreferences {
        terminal: terminal::read_preference().id().to_string(),
        options,
        workspace,
        workspace_options: Vec::new(),
        selected_agent: selected_agent.clone(),
        agent_preferences,
        default_agent,
        default_profile_id,
        enabled_agents: cfg.enabled_agents.clone(),
        default_profiles,
        compatibility_proxy: terminal::read_compatibility_proxy_preference(),
        profile_connections: merged_profile_connections(&agent_prefs),
    }
}

#[tauri::command]
pub async fn launcher_list_workspaces(
    agent_id: Option<String>,
) -> Result<Vec<WorkspaceOption>, String> {
    tauri::async_runtime::spawn_blocking(move || launcher_workspace_options(agent_id.as_deref()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profiles_launch_default() -> Result<(), String> {
    let cfg = config::ensure_loaded();
    let agent_prefs = agent_state::read_prefs();
    let agent_id = agent_state::resolve_default_agent(&agent_prefs, &cfg);
    let profile_id = agent_state::resolve_default_profile(&agent_prefs, &cfg, &agent_id);
    if let Some(profile_id) = profile_id {
        if let Some(profile) = schema::load(&profile_id).map(normalize_legacy_profile) {
            if profile_can_launch_agent(&profile, &agent_id) {
                return launcher::launch(&profile, &agent_id).map_err(|e| e.to_string());
            }
        }
    }
    launcher::launch_direct(&agent_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profiles_launch_resume(
    id: String,
    launch_target: String,
    session_id: String,
) -> Result<(), String> {
    let profile = schema::load(&id)
        .map(normalize_legacy_profile)
        .ok_or_else(|| format!("profile '{id}' not found"))?;
    if !profile_can_launch_agent(&profile, &launch_target) {
        return Err(format!("profile '{id}' cannot launch '{launch_target}'"));
    }
    launcher::launch_resume(&profile, &launch_target, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profiles_launch_direct_resume(agent_id: String, session_id: String) -> Result<(), String> {
    let agent_id = canonical_agent_id(&agent_id);
    launcher::launch_direct_resume(&agent_id, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launcher_list_sessions(
    agent_id: String,
    workspace_path: String,
    include_archived: bool,
) -> Result<Vec<LaunchSessionSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let agent_id = canonical_agent_id(&agent_id);
        common::launch_sessions::list_for_agent_workspace_with_archived(
            &agent_id,
            Path::new(&workspace_path),
            25,
            include_archived,
        )
        .into_iter()
        .map(|session| LaunchSessionSummary {
            short_id: common::launch_sessions::short_id(&session.session_id),
            agent_id: session.agent_id,
            session_id: session.session_id,
            title: session.title,
            workspace: session.workspace,
            updated_at: session.updated_at,
            archived: session.archived,
        })
        .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launcher_set_default(
    app: tauri::AppHandle,
    agent_id: String,
    profile_id: Option<String>,
) -> Result<(), String> {
    let (agent_id, profile_id) = validate_agent_profile_selection(&agent_id, profile_id)?;
    agent_state::write_default_launch(&agent_id, profile_id).map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn launcher_set_agent_profile(
    app: tauri::AppHandle,
    agent_id: String,
    profile_id: Option<String>,
) -> Result<(), String> {
    let (agent_id, profile_id) = validate_agent_profile_selection(&agent_id, profile_id)?;
    agent_state::write_agent_profile(&agent_id, profile_id).map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

fn validate_agent_profile_selection(
    agent_id: &str,
    profile_id: Option<String>,
) -> Result<(String, Option<String>), String> {
    let agent_id = resources::agent_by_alias(agent_id)
        .map(|def| def.id.clone())
        .ok_or_else(|| format!("unknown agent: '{agent_id}'"))?;
    let profile_id = profile_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());

    if let Some(profile_id) = &profile_id {
        let profile = schema::load(profile_id)
            .map(normalize_legacy_profile)
            .ok_or_else(|| format!("profile '{profile_id}' not found"))?;
        if !profile_can_launch_agent(&profile, &agent_id) {
            return Err(format!("profile '{profile_id}' cannot launch '{agent_id}'"));
        }
    }

    Ok((agent_id, profile_id))
}

#[tauri::command]
pub fn launcher_set_selected_agent(app: tauri::AppHandle, agent_id: String) -> Result<(), String> {
    let agent_id = resources::agent_by_alias(&agent_id)
        .map(|def| def.id.clone())
        .ok_or_else(|| format!("unknown agent: '{agent_id}'"))?;
    agent_state::write_selected_agent(&agent_id).map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn launcher_set_terminal(terminal_id: String) -> Result<(), String> {
    let choice = terminal::TerminalChoice::from_id(&terminal_id)
        .ok_or_else(|| format!("unknown terminal: '{}'", terminal_id))?;
    if !terminal::TerminalChoice::ALL.contains(&choice) {
        return Err(format!(
            "terminal '{}' is not supported on this platform",
            terminal_id
        ));
    }
    terminal::write_preference(choice).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launcher_set_workspace(
    app: tauri::AppHandle,
    workspace_path: String,
    agent_id: Option<String>,
) -> Result<(), String> {
    let path = terminal::canonical_workspace_path(Path::new(&workspace_path))
        .map_err(|e| e.to_string())?;
    register_launcher_workspace(&path)?;
    let cfg = config::ensure_loaded();
    let agent_prefs = agent_state::read_prefs();
    let agent_id = agent_id
        .map(|id| canonical_agent_id(&id))
        .unwrap_or_else(|| agent_state::resolve_selected_agent(&agent_prefs, &cfg));
    agent_state::write_agent_workspace(&agent_id, path).map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn launcher_remove_workspace(
    app: tauri::AppHandle,
    workspace_path: String,
) -> Result<(), String> {
    let path = PathBuf::from(workspace_path);
    let cfg = config::ensure_loaded();
    let builtin = config::builtin_workspaces_dir();
    if paths_equal(&path, &builtin) {
        return Err("Cannot remove the built-in workspace".to_string());
    }
    if !cfg
        .workspaces
        .iter()
        .any(|workspace| paths_equal(workspace, &path))
    {
        return Err(format!("workspace is not registered: {}", path.display()));
    }

    config::update_settings_json(|root| {
        if let Some(arr) = root
            .get_mut("workspaces")
            .and_then(|value| value.as_array_mut())
        {
            arr.retain(|value| {
                value
                    .as_str()
                    .map(|candidate| !paths_equal(Path::new(candidate), &path))
                    .unwrap_or(true)
            });
        }
    })
    .map_err(|e| e.to_string())?;

    if terminal::read_workspace_preference()
        .as_ref()
        .map(|selected| paths_equal(selected, &path))
        .unwrap_or(false)
    {
        let fallback = config::ensure_loaded().resolve_workspace("codex");
        terminal::write_workspace_preference(fallback).map_err(|e| e.to_string())?;
    }
    agent_state::remove_workspace_references(&path).map_err(|e| e.to_string())?;

    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn launcher_reorder_workspaces(
    app: tauri::AppHandle,
    workspace_paths: Vec<String>,
) -> Result<(), String> {
    let cfg = config::ensure_loaded();
    let builtin = config::builtin_workspaces_dir();
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();

    for path in workspace_paths {
        let canonical = PathBuf::from(path);
        if paths_equal(&canonical, &builtin) {
            continue;
        }
        if cfg
            .workspaces
            .iter()
            .any(|workspace| paths_equal(workspace, &canonical))
            && seen.insert(canonical.clone())
        {
            ordered.push(canonical);
        }
    }

    for workspace in &cfg.workspaces {
        if paths_equal(workspace, &builtin) {
            continue;
        }
        if seen.insert(workspace.clone()) {
            ordered.push(workspace.clone());
        }
    }

    let mut final_order = Vec::new();
    final_order.extend(ordered);

    config::update_settings_json(|root| {
        if let Some(obj) = root.as_object_mut() {
            obj.insert(
                "workspaces".into(),
                serde_json::Value::Array(
                    final_order
                        .iter()
                        .map(|path| serde_json::Value::String(path.to_string_lossy().to_string()))
                        .collect(),
                ),
            );
        }
    })
    .map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn launcher_set_compatibility_proxy(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    let mode = terminal::CompatibilityProxyMode::from_id(&mode)
        .ok_or_else(|| format!("unknown compatibility proxy mode: '{mode}'"))?;
    terminal::write_compatibility_proxy_preference(mode).map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn launcher_set_profile_connection(
    app: tauri::AppHandle,
    profile_id: String,
    agent_id: String,
    preference: agent_state::ProfileConnectionPreference,
) -> Result<(), String> {
    let agent_id = match agent_id.as_str() {
        "claude" | "codex" | "opencode" => agent_id,
        other => return Err(format!("unsupported connection target: '{other}'")),
    };
    let profile = schema::load(&profile_id)
        .map(normalize_legacy_profile)
        .ok_or_else(|| format!("profile '{profile_id}' not found"))?;
    let preference = sanitize_profile_connection_preference(&profile, &agent_id, preference)?;

    agent_state::write_profile_connection_preference(&profile.id, &agent_id, preference)
        .map_err(|e| e.to_string())?;
    emit_launch_config_changed(&app);
    Ok(())
}

pub(super) fn is_proxy_target_api_type(api_type: &str) -> bool {
    matches!(api_type, "anthropic" | "openai-responses" | "openai-chat")
}

pub(super) fn recommended_proxy_target(
    api_types: &[String],
    agent_id: &str,
    client_api_type: &str,
) -> Option<String> {
    let order: &[&str] = match (agent_id, client_api_type) {
        ("claude", "anthropic") | ("opencode", "anthropic") => {
            &["openai-responses", "openai-chat", "anthropic"]
        }
        ("codex", "openai-responses")
        | ("opencode", "openai-responses")
        | ("opencode", "openai-chat") => &["anthropic", "openai-chat", "openai-responses"],
        _ => &[],
    };
    order
        .iter()
        .find(|candidate| api_types.iter().any(|api_type| api_type == *candidate))
        .map(|candidate| (*candidate).to_string())
}

fn sanitize_profile_connection_preference(
    profile: &ProfileDef,
    agent_id: &str,
    preference: agent_state::ProfileConnectionPreference,
) -> Result<agent_state::ProfileConnectionPreference, String> {
    let supported = agent_client_api_types(agent_id);
    let selected_api_type = preference
        .selected_api_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| recommended_client_api_type(profile, agent_id).unwrap_or(supported[0]));
    if !supported.contains(&selected_api_type) {
        return Err(format!(
            "{} does not support api kind '{}'",
            agent_id, selected_api_type
        ));
    }

    let mut proxy = BTreeMap::new();
    for (client_api_type, proxy_preference) in preference.proxy {
        let client_api_type = client_api_type.trim().to_string();
        if client_api_type.is_empty() {
            continue;
        }
        if !supported.contains(&client_api_type.as_str()) {
            return Err(format!(
                "{} does not support api kind '{}'",
                agent_id, client_api_type
            ));
        }
        let target_api_type = proxy_preference
            .target_api_type
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let target_api_type = if proxy_preference.enabled {
            let target_api_type = target_api_type.or_else(|| {
                recommended_proxy_target(&profile.api_types, agent_id, &client_api_type)
            });
            let target_api_type = target_api_type.ok_or_else(|| {
                format!(
                    "profile '{}' has no API kind that can be used as a proxy target",
                    profile.id
                )
            })?;
            validate_proxy_target(profile, &target_api_type)?;
            Some(target_api_type)
        } else {
            target_api_type.filter(|api_type| validate_proxy_target(profile, api_type).is_ok())
        };
        let upstream_model = proxy_preference
            .upstream_model
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let fake_model_id = proxy_preference
            .fake_model_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if proxy_preference.enabled
            || target_api_type.is_some()
            || upstream_model.is_some()
            || fake_model_id.is_some()
        {
            proxy.insert(
                client_api_type,
                agent_state::ProfileProxyPreference {
                    enabled: proxy_preference.enabled,
                    target_api_type,
                    upstream_model,
                    fake_model_id,
                },
            );
        }
    }

    Ok(agent_state::ProfileConnectionPreference {
        selected_api_type: Some(selected_api_type.to_string()),
        proxy,
    })
}

fn validate_proxy_target(profile: &ProfileDef, target_api_type: &str) -> Result<(), String> {
    if !profile
        .api_types
        .iter()
        .any(|api_type| api_type == target_api_type)
    {
        return Err(format!(
            "profile '{}' does not expose api kind '{}'",
            profile.id, target_api_type
        ));
    }
    if !is_proxy_target_api_type(target_api_type) {
        return Err(format!(
            "api kind '{}' cannot be used as a proxy target",
            target_api_type
        ));
    }
    Ok(())
}

fn profile_can_launch_agent(profile: &ProfileDef, agent_id: &str) -> bool {
    resolve_profile_agent_route(profile, agent_id).is_some()
}

#[derive(Debug, Clone)]
pub(super) struct ProfileAgentRoute {
    pub client_api_type: String,
    pub proxy_target_api_type: Option<String>,
    pub proxy_upstream_model: Option<String>,
    pub proxy_fake_model_id: Option<String>,
}

pub(super) fn resolve_profile_agent_route(
    profile: &ProfileDef,
    agent_id: &str,
) -> Option<ProfileAgentRoute> {
    let supported = agent_client_api_types(agent_id);
    if supported.is_empty() {
        return None;
    }

    let connections = merged_profile_connections(&agent_state::read_prefs());
    let preference = connections
        .get(&profile.id)
        .and_then(|items| items.get(agent_id));
    let preferred_client_api_type = preference
        .and_then(|preference| preference.selected_api_type.as_deref())
        .filter(|api_type| supported.contains(api_type))
        .filter(|api_type| client_route_available(profile, agent_id, preference, api_type))
        .map(ToString::to_string);
    let client_api_type = preferred_client_api_type
        .or_else(|| recommended_client_api_type(profile, agent_id).map(ToString::to_string))?;

    let proxy_preference = preference.and_then(|preference| preference.proxy.get(&client_api_type));
    if let Some(proxy_preference) = proxy_preference.filter(|proxy| proxy.enabled) {
        let target_api_type = proxy_preference
            .target_api_type
            .clone()
            .or_else(|| recommended_proxy_target(&profile.api_types, agent_id, &client_api_type))?;
        if validate_proxy_target(profile, &target_api_type).is_ok() {
            return Some(ProfileAgentRoute {
                client_api_type,
                proxy_target_api_type: Some(target_api_type),
                proxy_upstream_model: proxy_preference.upstream_model.clone(),
                proxy_fake_model_id: proxy_preference.fake_model_id.clone(),
            });
        }
    }

    if profile
        .api_types
        .iter()
        .any(|api_type| api_type == &client_api_type)
    {
        return Some(ProfileAgentRoute {
            client_api_type,
            proxy_target_api_type: None,
            proxy_upstream_model: None,
            proxy_fake_model_id: None,
        });
    }

    None
}

fn client_route_available(
    profile: &ProfileDef,
    agent_id: &str,
    preference: Option<&agent_state::ProfileConnectionPreference>,
    client_api_type: &str,
) -> bool {
    if profile
        .api_types
        .iter()
        .any(|api_type| api_type == client_api_type)
    {
        return true;
    }
    let Some(proxy_preference) =
        preference.and_then(|preference| preference.proxy.get(client_api_type))
    else {
        return false;
    };
    if !proxy_preference.enabled {
        return false;
    }
    let Some(target_api_type) = proxy_preference
        .target_api_type
        .clone()
        .or_else(|| recommended_proxy_target(&profile.api_types, agent_id, client_api_type))
    else {
        return false;
    };
    validate_proxy_target(profile, &target_api_type).is_ok()
}

pub(super) fn agent_client_api_types(agent_id: &str) -> &'static [&'static str] {
    match agent_id {
        "claude" => &["anthropic"],
        "codex" => &["openai-responses"],
        "opencode" => &["openai-responses", "openai-chat", "anthropic"],
        _ => &[],
    }
}

fn recommended_client_api_type(profile: &ProfileDef, agent_id: &str) -> Option<&'static str> {
    agent_client_api_types(agent_id)
        .iter()
        .find(|api_type| profile.api_types.iter().any(|value| value == *api_type))
        .copied()
        .or_else(|| agent_client_api_types(agent_id).first().copied())
}

fn launcher_workspace_options(agent_id: Option<&str>) -> Vec<WorkspaceOption> {
    let builtin = config::builtin_workspaces_dir();
    let home = terminal::launch_home_dir().unwrap_or_else(|_| config::data_dir());
    let agent_prefs = agent_state::read_prefs();
    let selected = agent_id
        .map(canonical_agent_id)
        .and_then(|agent_id| resolve_agent_workspace_preference(&agent_id, &agent_prefs).ok())
        .or_else(|| terminal::resolve_workspace_preference().ok());
    if let Some(path) = selected.as_ref() {
        let _ = register_launcher_workspace(path);
    }
    let cfg = config::ensure_loaded();

    let mut out = Vec::new();
    push_workspace_option(&mut out, &home, "Home", "home", false);
    for workspace in cfg.all_workspaces() {
        let is_default = paths_equal(&workspace, &builtin);
        let kind = if paths_equal(&workspace, &builtin) {
            "built-in"
        } else {
            "workspace"
        };
        let label = if is_default {
            "Default workspace".to_string()
        } else {
            path_label(&workspace)
        };
        push_workspace_option(&mut out, &workspace, &label, kind, is_default);
    }
    if let Some(path) = selected {
        if !out
            .iter()
            .any(|option| paths_equal(Path::new(&option.path), &path))
        {
            let label = path_label(&path);
            push_workspace_option(&mut out, &path, &label, "selected", false);
        }
    }
    out
}

fn canonical_agent_id(agent_id: &str) -> String {
    resources::agent_by_alias(agent_id)
        .map(|def| def.id.clone())
        .unwrap_or_else(|| agent_id.to_string())
}

fn resolve_agent_workspace_preference(
    agent_id: &str,
    agent_prefs: &agent_state::AgentsPrefsFile,
) -> anyhow::Result<PathBuf> {
    if let Some(workspace) = agent_prefs
        .agents
        .get(agent_id)
        .and_then(|preference| preference.workspace.as_ref())
    {
        return terminal::canonical_workspace_path(workspace);
    }
    terminal::resolve_workspace_preference()
}

pub(super) fn resolve_launch_workspace(agent_id: &str) -> anyhow::Result<PathBuf> {
    let agent_prefs = agent_state::read_prefs();
    resolve_agent_workspace_preference(agent_id, &agent_prefs)
}

fn summarize_agent_preferences(
    agent_prefs: &agent_state::AgentsPrefsFile,
    cfg: &config::Config,
) -> BTreeMap<String, AgentLaunchPreferenceSummary> {
    let mut agent_ids: HashSet<String> = cfg
        .enabled_agents
        .iter()
        .map(|id| canonical_agent_id(id))
        .collect();
    agent_ids.extend(agent_prefs.agents.keys().map(|id| canonical_agent_id(id)));
    agent_ids.extend(cfg.default_profiles.keys().map(|id| canonical_agent_id(id)));

    let mut out = BTreeMap::new();
    for agent_id in agent_ids {
        let stored = agent_prefs.agents.get(&agent_id);
        let profile_id = stored
            .and_then(|preference| preference.profile_id.clone())
            .or_else(|| cfg.default_profiles.get(&agent_id).cloned());
        let workspace = stored
            .and_then(|preference| preference.workspace.as_ref())
            .map(|path| path.to_string_lossy().to_string());
        if profile_id.is_some() || workspace.is_some() {
            out.insert(
                agent_id,
                AgentLaunchPreferenceSummary {
                    profile_id,
                    workspace,
                },
            );
        }
    }
    out
}

fn merged_profile_connections(
    agent_prefs: &agent_state::AgentsPrefsFile,
) -> agent_state::ProfileConnectionPreferences {
    let mut out = legacy_profile_connections();
    for (profile_id, by_agent) in &agent_prefs.profile_connections {
        let entry = out.entry(profile_id.clone()).or_default();
        for (agent_id, preference) in by_agent {
            entry.insert(agent_id.clone(), preference.clone());
        }
    }
    out
}

fn legacy_profile_connections() -> agent_state::ProfileConnectionPreferences {
    let legacy = terminal::read_profile_connections();
    let mut out = agent_state::ProfileConnectionPreferences::new();
    for (profile_id, by_agent) in legacy {
        let entry = out.entry(profile_id).or_default();
        for (agent_id, preference) in by_agent {
            let Some(selected_api_type) = default_client_api_type(&agent_id) else {
                continue;
            };
            let mut proxy = BTreeMap::new();
            if preference.proxy_enabled || preference.target_api_type.is_some() {
                proxy.insert(
                    selected_api_type.to_string(),
                    agent_state::ProfileProxyPreference {
                        enabled: preference.proxy_enabled,
                        target_api_type: preference.target_api_type,
                        upstream_model: None,
                        fake_model_id: None,
                    },
                );
            }
            entry.insert(
                agent_id,
                agent_state::ProfileConnectionPreference {
                    selected_api_type: Some(selected_api_type.to_string()),
                    proxy,
                },
            );
        }
    }
    out
}

fn default_client_api_type(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "claude" => Some("anthropic"),
        "codex" => Some("openai-responses"),
        "opencode" => Some("openai-responses"),
        _ => None,
    }
}

pub(crate) fn ordered_profiles() -> Vec<ProfileDef> {
    let mut remaining: Vec<_> = schema::list()
        .into_iter()
        .map(normalize_legacy_profile)
        .collect();
    let mut out = Vec::new();

    for id in read_profile_order() {
        if let Some(index) = remaining.iter().position(|profile| profile.id == id) {
            out.push(remaining.remove(index));
        }
    }

    out.extend(remaining);
    out
}

fn clear_default_profile_references(profile_id: &str) -> Result<(), String> {
    config::update_settings_json(|root| {
        if let Some(obj) = root.as_object_mut() {
            let mut remove_default_profiles = false;
            if let Some(map) = obj
                .get_mut("default_profiles")
                .and_then(|value| value.as_object_mut())
            {
                map.retain(|_, value| value.as_str() != Some(profile_id));
                remove_default_profiles = map.is_empty();
            }
            if remove_default_profiles {
                obj.remove("default_profiles");
            }
            if let Some(order) = obj
                .get_mut("profile_order")
                .and_then(|value| value.as_array_mut())
            {
                order.retain(|value| value.as_str() != Some(profile_id));
                if order.is_empty() {
                    obj.remove("profile_order");
                }
            }
        }
    })
    .map_err(|e| e.to_string())
}

fn read_profile_order() -> Vec<String> {
    let path = config::data_dir().join("settings.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
        .and_then(|root| {
            root.get("profile_order")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(str::trim)
                        .filter(|id| !id.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
        })
        .unwrap_or_default()
}

fn write_profile_order(profile_ids: &[String]) -> Result<(), String> {
    config::update_settings_json(|root| {
        if let Some(obj) = root.as_object_mut() {
            obj.insert(
                "profile_order".to_string(),
                serde_json::Value::Array(
                    profile_ids
                        .iter()
                        .map(|id| serde_json::Value::String(id.clone()))
                        .collect(),
                ),
            );
        }
    })
    .map_err(|e| e.to_string())
}

fn ensure_profile_order_contains(profile_id: &str) -> Result<(), String> {
    let mut order = read_profile_order();
    if !order.iter().any(|id| id == profile_id) {
        order.push(profile_id.to_string());
        write_profile_order(&order)?;
    }
    Ok(())
}

fn emit_launch_config_changed(app: &tauri::AppHandle) {
    let _ = app.emit(crate::tray::LAUNCH_CONFIG_CHANGED_EVENT, ());
}

fn register_launcher_workspace(path: &Path) -> Result<(), String> {
    let builtin = config::builtin_workspaces_dir();
    if paths_equal(path, &builtin) {
        return Ok(());
    }
    if terminal::launch_home_dir()
        .as_ref()
        .map(|home| paths_equal(path, home))
        .unwrap_or(false)
    {
        return Ok(());
    }

    let cfg = config::ensure_loaded();
    if cfg
        .workspaces
        .iter()
        .any(|workspace| paths_equal(workspace, path))
    {
        return Ok(());
    }

    config::update_settings_json(|root| {
        if !root.is_object() {
            *root = serde_json::json!({});
        }
        if let Some(obj) = root.as_object_mut() {
            let workspaces = obj
                .entry("workspaces")
                .or_insert_with(|| serde_json::json!([]));
            if !workspaces.is_array() {
                *workspaces = serde_json::json!([]);
            }
            if let Some(arr) = workspaces.as_array_mut() {
                let already_registered = arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .any(|candidate| paths_equal(Path::new(candidate), path));
                if !already_registered {
                    arr.push(serde_json::Value::String(
                        path.to_string_lossy().to_string(),
                    ));
                }
            }
        }
    })
    .map_err(|e| e.to_string())
}

fn push_workspace_option(
    out: &mut Vec<WorkspaceOption>,
    path: &Path,
    label: &str,
    kind: &str,
    is_default: bool,
) {
    if out
        .iter()
        .any(|option| paths_equal(Path::new(&option.path), path))
    {
        return;
    }
    out.push(WorkspaceOption {
        path: path.to_string_lossy().to_string(),
        label: label.to_string(),
        detail: path.to_string_lossy().to_string(),
        kind: kind.to_string(),
        is_default,
    });
}

fn path_label(path: &Path) -> String {
    if let Some(name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
    {
        name.to_string()
    } else {
        path.to_string_lossy().to_string()
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
        || std::fs::canonicalize(left)
            .ok()
            .zip(std::fs::canonicalize(right).ok())
            .map(|(left, right)| left == right)
            .unwrap_or(false)
}
