//! Install orchestration: runs agents + plugin installs with progress reporting.

use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex;

use super::{plugin_install, write_settings_value, InstallProgressEvent, InstallTaskInfo, OnboardingInstallState};
use common::config;

/// Returns the list of install tasks for the given settings, so the frontend
/// can pre-populate the progress list before install starts.
pub fn get_install_manifest(settings: &Value) -> Vec<InstallTaskInfo> {
    let all_agents = common::resources::agent_ids();
    let enabled_agents = resolve_enabled_agents(settings, &all_agents);

    let mut tasks = Vec::new();

    for agent_id in &enabled_agents {
        let agent_def = match common::resources::agent_by_id(agent_id) {
            Some(def) => def,
            None => continue,
        };

        // MCP config + skill are always installed
        if agent_def.global_config.is_some() {
            tasks.push(InstallTaskInfo {
                id: format!("agent:{}:mcp", agent_id),
                label: format!("{} — MCP config", agent_def.display_name),
            });
            if agent_def.global_config.as_ref().and_then(|c| c.skill_dir.as_ref()).is_some() {
                tasks.push(InstallTaskInfo {
                    id: format!("agent:{}:skill", agent_id),
                    label: format!("{} — Skill file", agent_def.display_name),
                });
            }
        }

        // ACP agent install (npm or script) — only for installable types
        let install_type = agent_def.install.as_ref().map(|i| i.install_type.as_str());
        if matches!(install_type, Some("npm") | Some("script")) {
            tasks.push(InstallTaskInfo {
                id: format!("agent:{}:acp", agent_id),
                label: format!("{} — CLI install", agent_def.display_name),
            });
        }
    }

    // Channel plugins
    let enabled_channels = settings
        .get("channels")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for channel_id in &enabled_channels {
        let plugin_def = common::resources::plugin_by_id(channel_id);
        let label = plugin_def
            .map(|p| p.name.clone())
            .unwrap_or_else(|| channel_id.clone());
        tasks.push(InstallTaskInfo {
            id: format!("plugin:{}", channel_id),
            label: format!("{} — Plugin install", label),
        });
    }

    tasks
}

/// Start the install, saving settings first, then spawning the background task.
pub async fn start<R: Runtime>(
    app: AppHandle<R>,
    install_state: &OnboardingInstallState,
    settings: Value,
) -> Result<(), String> {
    install_state.cancelled.store(false, Ordering::Relaxed);

    // Save settings with onboarded: true
    let mut val = settings;
    if let Some(obj) = val.as_object_mut() {
        obj.insert("onboarded".into(), serde_json::json!(true));
    }
    write_settings_value(&val)?;

    // Create log file
    let log_dir = config::data_dir().join("logs").join("onboarding");
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let log_path = log_dir.join(format!("{}.log", timestamp));
    let log_file = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    {
        let mut lf = install_state.log_file.lock().await;
        *lf = Some(log_file);
    }

    let cancelled = Arc::clone(&install_state.cancelled);
    let child_proc = Arc::clone(&install_state.child_process);
    let log_file_arc = Arc::clone(&install_state.log_file);

    tauri::async_runtime::spawn(async move {
        run_install(app, val, cancelled, child_proc, log_file_arc).await;
    });

    Ok(())
}

/// Cancel a running install.
pub async fn cancel(install_state: &OnboardingInstallState) -> Result<(), String> {
    install_state.cancelled.store(true, Ordering::Relaxed);
    let mut child = install_state.child_process.lock().await;
    if let Some(ref mut proc) = *child {
        let _ = proc.kill().await;
    }
    *child = None;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

fn emit_progress<R: Runtime>(app: &AppHandle<R>, event: &InstallProgressEvent) {
    let _ = app.emit("onboarding-install-progress", event);
}

fn log_line(log_file: &Arc<Mutex<Option<std::fs::File>>>, line: &str) {
    if let Ok(mut guard) = log_file.try_lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "{}", line);
        }
    }
}

fn resolve_enabled_agents(settings: &Value, all_agents: &[&str]) -> Vec<String> {
    common::agent_integrations::resolve_enabled_agents(settings, all_agents)
}

async fn run_install<R: Runtime>(
    app: AppHandle<R>,
    settings: Value,
    cancelled: Arc<AtomicBool>,
    _child_proc: Arc<Mutex<Option<tokio::process::Child>>>,
    log_file: Arc<Mutex<Option<std::fs::File>>>,
) {
    let all_agents = common::resources::agent_ids();
    let enabled_agents = resolve_enabled_agents(&settings, &all_agents);
    let mut had_error = false;

    // Install MCP config + skill files for all enabled agents in one global
    // sweep BEFORE the per-agent loop.
    if !enabled_agents.is_empty() {
        log_line(&log_file, "[onboarding] Running sync_integrations (global MCP + skills sweep)");
        common::agent_integrations::sync_integrations(&settings);
    }

    // --- Agent installs ---
    for agent_id in &enabled_agents {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        let agent_def = match common::resources::agent_by_id(agent_id) {
            Some(def) => def,
            None => continue,
        };

        // MCP config
        if agent_def.global_config.is_some() {
            let task_id = format!("agent:{}:mcp", agent_id);
            emit_progress(&app, &InstallProgressEvent {
                id: task_id.clone(),
                label: format!("{} — MCP config", agent_def.display_name),
                status: "running".into(),
                message: Some("Installing MCP config…".into()),
            });
            log_line(&log_file, &format!("[{}] Installing MCP config", agent_id));

            emit_progress(&app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — MCP config", agent_def.display_name),
                status: "done".into(),
                message: None,
            });

            if agent_def.global_config.as_ref().and_then(|c| c.skill_dir.as_ref()).is_some() {
                let skill_id = format!("agent:{}:skill", agent_id);
                emit_progress(&app, &InstallProgressEvent {
                    id: skill_id,
                    label: format!("{} — Skill file", agent_def.display_name),
                    status: "done".into(),
                    message: None,
                });
            }
        }

        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        // ACP agent install (npm or script)
        let install_type = agent_def.install.as_ref().map(|i| i.install_type.as_str());
        match install_type {
            Some("npm") => install_npm_agent(&app, agent_id, agent_def, &log_file, &mut had_error).await,
            Some("script") => install_script_agent(&app, agent_id, agent_def, &log_file, &mut had_error).await,
            _ => {} // "path" type — nothing to install
        }
    }

    // --- Channel plugin installs ---
    let enabled_channels = settings
        .get("channels")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for channel_id in &enabled_channels {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        install_channel_plugin(&app, channel_id, &log_file, &mut had_error).await;
    }

    // Emit final complete event
    let final_status = if cancelled.load(Ordering::Relaxed) {
        "cancelled"
    } else if had_error {
        "error"
    } else {
        "complete"
    };

    let _ = app.emit("onboarding-install-complete", serde_json::json!({
        "status": final_status,
    }));

    let mut lf = log_file.lock().await;
    *lf = None;
}

async fn install_npm_agent<R: Runtime>(
    app: &AppHandle<R>,
    agent_id: &str,
    agent_def: &common::resources::AgentDef,
    log_file: &Arc<Mutex<Option<std::fs::File>>>,
    had_error: &mut bool,
) {
    let task_id = format!("agent:{}:acp", agent_id);
    let Some(npm_pkg) = &agent_def.acp.npm_package else { return };
    let bin_name = agent_def.acp.bin_name.as_deref().unwrap_or(npm_pkg);

    if common::env::resolve_acp_agent_bin(bin_name).is_ok() {
        emit_progress(app, &InstallProgressEvent {
            id: task_id,
            label: format!("{} — CLI install", agent_def.display_name),
            status: "skipped".into(),
            message: Some("Already installed".into()),
        });
        log_line(log_file, &format!("[{}] ACP agent already installed, skipping", agent_id));
        return;
    }

    let msg = format!("Running: npm install {}", npm_pkg);
    emit_progress(app, &InstallProgressEvent {
        id: task_id.clone(),
        label: format!("{} — CLI install", agent_def.display_name),
        status: "running".into(),
        message: Some(msg.clone()),
    });
    log_line(log_file, &format!("[{}] {}", agent_id, msg));

    match common::agent_integrations::auto_install_npm_agent_with_output(npm_pkg).await {
        Ok(out) => {
            log_line(log_file, &format!("[{}] stdout:\n{}", agent_id, out.stdout));
            log_line(log_file, &format!("[{}] stderr:\n{}", agent_id, out.stderr));
            emit_progress(app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — CLI install", agent_def.display_name),
                status: "done".into(),
                message: None,
            });
            log_line(log_file, &format!("[{}] npm install complete", agent_id));
        }
        Err(e) => {
            *had_error = true;
            let err_msg = format!("{:#}", e);
            emit_progress(app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — CLI install", agent_def.display_name),
                status: "error".into(),
                message: Some(err_msg.clone()),
            });
            log_line(log_file, &format!("[{}] ERROR: {}", agent_id, err_msg));
        }
    }
}

async fn install_script_agent<R: Runtime>(
    app: &AppHandle<R>,
    agent_id: &str,
    agent_def: &common::resources::AgentDef,
    log_file: &Arc<Mutex<Option<std::fs::File>>>,
    had_error: &mut bool,
) {
    let task_id = format!("agent:{}:acp", agent_id);

    if common::agent_integrations::is_program_available(&agent_def.acp.program) {
        emit_progress(app, &InstallProgressEvent {
            id: task_id,
            label: format!("{} — CLI install", agent_def.display_name),
            status: "skipped".into(),
            message: Some("Already installed".into()),
        });
        log_line(log_file, &format!("[{}] CLI already available in PATH, skipping", agent_id));
        return;
    }

    let Some(install_cmd) = &agent_def.acp.install_cmd else { return };
    let msg = format!("Running: {}", install_cmd);
    emit_progress(app, &InstallProgressEvent {
        id: task_id.clone(),
        label: format!("{} — CLI install", agent_def.display_name),
        status: "running".into(),
        message: Some(msg.clone()),
    });
    log_line(log_file, &format!("[{}] {}", agent_id, msg));

    match common::agent_integrations::auto_install_agent_cmd_with_output(install_cmd, agent_id).await {
        Ok(out) => {
            log_line(log_file, &format!("[{}] stdout:\n{}", agent_id, out.stdout));
            log_line(log_file, &format!("[{}] stderr:\n{}", agent_id, out.stderr));
            emit_progress(app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — CLI install", agent_def.display_name),
                status: "done".into(),
                message: None,
            });
            log_line(log_file, &format!("[{}] script install complete", agent_id));
        }
        Err(e) => {
            *had_error = true;
            let err_msg = format!("{:#}", e);
            emit_progress(app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — CLI install", agent_def.display_name),
                status: "error".into(),
                message: Some(err_msg.clone()),
            });
            log_line(log_file, &format!("[{}] ERROR: {}", agent_id, err_msg));
        }
    }
}

async fn install_channel_plugin<R: Runtime>(
    app: &AppHandle<R>,
    channel_id: &str,
    log_file: &Arc<Mutex<Option<std::fs::File>>>,
    had_error: &mut bool,
) {
    let task_id = format!("plugin:{}", channel_id);
    let plugin_def = common::resources::plugin_by_id(channel_id);
    let label = plugin_def
        .map(|p| p.name.clone())
        .unwrap_or_else(|| channel_id.to_string());

    // Check if already ready
    let status = plugin_install::check_plugin_status(channel_id.to_string());
    if status == "ready" {
        emit_progress(app, &InstallProgressEvent {
            id: task_id,
            label: format!("{} — Plugin install", label),
            status: "skipped".into(),
            message: Some("Already installed".into()),
        });
        log_line(log_file, &format!("[plugin:{}] Already ready, skipping", channel_id));
        return;
    }

    let github_url = match plugin_def {
        Some(p) => p.github.clone(),
        None => {
            emit_progress(app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — Plugin install", label),
                status: "error".into(),
                message: Some("Plugin not found in registry".into()),
            });
            *had_error = true;
            return;
        }
    };

    emit_progress(app, &InstallProgressEvent {
        id: task_id.clone(),
        label: format!("{} — Plugin install", label),
        status: "running".into(),
        message: Some("Running: git clone + npm install + build".into()),
    });
    log_line(log_file, &format!("[plugin:{}] Starting install from {}", channel_id, github_url));

    let request = plugin_install::InstallPluginRequest {
        plugin_id: channel_id.to_string(),
        github_url,
    };
    match plugin_install::run_install_inner(request).await {
        Ok(resp) => {
            if resp.success {
                emit_progress(app, &InstallProgressEvent {
                    id: task_id,
                    label: format!("{} — Plugin install", label),
                    status: "done".into(),
                    message: None,
                });
                log_line(log_file, &format!("[plugin:{}] Install complete", channel_id));
            } else {
                *had_error = true;
                emit_progress(app, &InstallProgressEvent {
                    id: task_id,
                    label: format!("{} — Plugin install", label),
                    status: "error".into(),
                    message: Some(resp.message),
                });
            }
        }
        Err(e) => {
            *had_error = true;
            let err_msg = format!("{:#}", e);
            emit_progress(app, &InstallProgressEvent {
                id: task_id,
                label: format!("{} — Plugin install", label),
                status: "error".into(),
                message: Some(err_msg.clone()),
            });
            log_line(log_file, &format!("[plugin:{}] ERROR: {}", channel_id, err_msg));
        }
    }
}
