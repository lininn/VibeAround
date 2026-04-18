use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use tokio::io::DuplexStream;
use tokio::sync::mpsc;

/// External CLI/provider session identifier.
pub type ProviderSessionId = String;

/// Low-level ACP transport connection returned by a provider wrapper.
pub struct ProviderConnection {
    pub read_stream: DuplexStream,
    pub write_stream: DuplexStream,
    pub session_id_rx: Option<mpsc::UnboundedReceiver<ProviderSessionId>>,
    pub worker_thread: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum AgentKind {
    Claude,
    Gemini,
    #[serde(rename = "opencode")]
    OpenCode,
    Codex,
    Cursor,
    Kiro,
    QwenCode,
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Gemini => write!(f, "gemini"),
            Self::OpenCode => write!(f, "opencode"),
            Self::Codex => write!(f, "codex"),
            Self::Cursor => write!(f, "cursor"),
            Self::Kiro => write!(f, "kiro"),
            Self::QwenCode => write!(f, "qwen-code"),
        }
    }
}

impl AgentKind {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        // Look up by alias in resources, then map the agent ID to the enum variant
        let agent = crate::resources::agent_by_alias(s)?;
        Self::from_id(&agent.id)
    }

    /// Map an agent ID string to the enum variant.
    fn from_id(id: &str) -> Option<Self> {
        match id {
            "claude" => Some(Self::Claude),
            "gemini" => Some(Self::Gemini),
            "opencode" => Some(Self::OpenCode),
            "codex" => Some(Self::Codex),
            "cursor" => Some(Self::Cursor),
            "kiro" => Some(Self::Kiro),
            "qwen-code" => Some(Self::QwenCode),
            _ => None,
        }
    }

    pub fn all() -> &'static [AgentKind] {
        &[Self::Claude, Self::Gemini, Self::OpenCode, Self::Codex, Self::Cursor, Self::Kiro, Self::QwenCode]
    }

    pub fn enabled() -> Vec<AgentKind> {
        crate::config::ensure_loaded().enabled_agents.clone()
    }

    pub fn is_enabled(&self) -> bool {
        crate::config::ensure_loaded().enabled_agents.contains(self)
    }

    pub fn display_name(&self) -> &str {
        crate::resources::agent_by_id(&self.to_string())
            .expect("AgentKind variant missing from agents.json")
            .display_name.as_str()
    }

    pub fn description(&self) -> &str {
        crate::resources::agent_by_id(&self.to_string())
            .expect("AgentKind variant missing from agents.json")
            .description.as_str()
    }
}

// ---------------------------------------------------------------------------
// AgentProvider trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AgentProvider: Send + Sync {
    fn kind(&self) -> AgentKind;

    async fn connect(
        &self,
        workspace: &Path,
        extra_env: &[(&str, &str)],
    ) -> anyhow::Result<ProviderConnection>;
}

pub fn provider_for_kind(kind: AgentKind) -> Arc<dyn AgentProvider> {
    Arc::new(StdioAcpProvider::new(kind))
}

// ---------------------------------------------------------------------------
// StdioAcpProvider — generic provider for CLIs that speak ACP over stdio
// ---------------------------------------------------------------------------

struct StdioAcpProvider {
    agent_kind: AgentKind,
}

impl StdioAcpProvider {
    fn new(kind: AgentKind) -> Self {
        Self { agent_kind: kind }
    }
}

#[async_trait]
impl AgentProvider for StdioAcpProvider {
    fn kind(&self) -> AgentKind { self.agent_kind }

    async fn connect(
        &self,
        workspace: &Path,
        extra_env: &[(&str, &str)],
    ) -> anyhow::Result<ProviderConnection> {
        let agent_def = crate::resources::agent_by_id(&self.agent_kind.to_string())
            .ok_or_else(|| anyhow!("No resource definition for agent '{}'", self.agent_kind))?;

        // Resolve program + args based on install method:
        // 1. npm-based agents → `node <resolved_entry>` (Claude ACP, Codex ACP)
        // 2. binary-download agents → binary from ~/.vibearound/bin/ (Cursor, Kiro)
        // 3. native agents → program + args from PATH (Gemini, OpenCode)
        let (program, resolved_args) = if let Some(npm_pkg) = &agent_def.acp.npm_package {
            let bin_name = agent_def.acp.bin_name.as_deref().unwrap_or(npm_pkg);
            if crate::env::resolve_acp_agent_bin(bin_name).is_err() {
                eprintln!("[{}-acp] auto-installing {} ...", self.agent_kind, npm_pkg);
                crate::agent_integrations::auto_install_npm_agent(npm_pkg).await?;
            }
            let entry = crate::env::resolve_acp_agent_bin(bin_name)
                .with_context(|| format!("Resolving ACP agent '{}' (npm: {})", self.agent_kind, npm_pkg))?;
            ("node".to_string(), vec![entry.to_string_lossy().to_string()])
        } else if let Some(install_cmd) = &agent_def.acp.install_cmd {
            if !crate::agent_integrations::is_program_available(&agent_def.acp.program) {
                eprintln!("[{}-acp] auto-installing via install cmd ...", self.agent_kind);
                crate::agent_integrations::auto_install_agent_cmd(install_cmd, &self.agent_kind.to_string()).await?;
            }
            (agent_def.acp.program.clone(), agent_def.acp.args.clone())
        } else {
            (agent_def.acp.program.clone(), agent_def.acp.args.clone())
        };

        let args_refs: Vec<&str> = resolved_args.iter().map(|s| s.as_str()).collect();
        let (read_stream, write_stream) =
            spawn_stdio_acp(self.agent_kind, &program, &args_refs, workspace, extra_env)?;
        Ok(ProviderConnection {
            read_stream,
            write_stream,
            session_id_rx: None,
            worker_thread: None,
        })
    }
}

/// Spawn a CLI that speaks ACP over stdio, return duplex streams.
fn spawn_stdio_acp(
    kind: AgentKind,
    program: &str,
    args: &[&str],
    cwd: &Path,
    extra_env: &[(&str, &str)],
) -> anyhow::Result<(DuplexStream, DuplexStream)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    eprintln!("[{}-acp] spawning {} {} in {:?}", kind, program, args.join(" "), cwd);
    let mut cmd = crate::env::command(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn()
        .with_context(|| format!("Failed to spawn {} {}. Is it installed?", program, args.join(" ")))?;
    eprintln!("[{}-acp] process spawned pid={:?}", kind, child.id());

    let child_stdout = child.stdout.take().context("Process has no stdout")?;
    let child_stdin = child.stdin.take().context("Process has no stdin")?;

    // Transfer ownership of `Child` to the global ChildRegistry. kill_on_drop
    // alone is not enough: the old code moved `child` into the stdout reader
    // closure, which only dropped it on stdout EOF. On abrupt runtime teardown
    // that task never ran its destructor, leaving PPID=1 orphans.
    // The registry's kill_all() path synchronously SIGKILLs every child on
    // daemon stop + Tauri Exit, regardless of task scheduler state.
    let registry_id = crate::child_registry::ChildRegistry::global().register(
        crate::child_registry::ChildKind::AgentAcp,
        format!("{}-acp", kind),
        child,
    );

    // stdout → client_read
    let (client_read, mut bridge_write) = tokio::io::duplex(64 * 1024);
    let kind_label = kind.to_string();
    tokio::task::spawn_local(async move {
        let mut stdout = child_stdout;
        let mut buf = [0u8; 8192];
        loop {
            match stdout.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if bridge_write.write_all(&buf[..n]).await.is_err() { break; }
                }
                Err(_) => break,
            }
        }
        // Clean shutdown path: pull the child out of the registry and drop
        // it. kill_on_drop fires if the process is still alive.
        if let Some(_c) = crate::child_registry::ChildRegistry::global().remove(registry_id) {
            eprintln!("[{}-acp] stdout EOF — dropping child via registry", kind_label);
        }
    });

    // client_write → stdin
    let (mut bridge_read, client_write) = tokio::io::duplex(64 * 1024);
    tokio::task::spawn_local(async move {
        let mut stdin = child_stdin;
        let mut buf = [0u8; 8192];
        loop {
            match bridge_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if stdin.write_all(&buf[..n]).await.is_err() { break; }
                    let _ = stdin.flush().await;
                }
                Err(_) => break,
            }
        }
    });

    Ok((client_read, client_write))
}
