//! Wire-facing snapshot types for `GET /api/services` and the
//! `/ws/services` WebSocket broadcast.
//!
//! These types exist only to be serialized for the dashboard; they are
//! a UX-shaped facade over the underlying domain state (tunnels,
//! channels, agent runtimes, PTY sessions). A future TUI / CLI should
//! not reuse them — it should read the underlying managers directly
//! and render in its own terms. Phase 1f will split this facade into
//! per-manager endpoints; these types are their transition home.
//!
//! Reference TS schema: `src/shared/client-ts/src/schemas.ts`.

use serde::Serialize;

use super::status::ServiceStatus;

/// Daemon metadata emitted on every services snapshot.
///
/// # Wire format (JSON)
/// ```json
/// { "started_at": 1713456789, "port": 12358 }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ServerMeta {
    pub started_at: u64,
    pub port: u16,
}

/// `GET /api/services` response (also broadcast on `/ws/services`).
///
/// # Wire format (JSON)
/// ```json
/// {
///   "server": { "started_at": 1713456789, "port": 12358 },
///   "tunnels":  [ /* ServiceInfo */ ],
///   "agents":   [ /* ServiceInfo */ ],
///   "channels": [ /* ServiceInfo */ ],
///   "pty_session_count": 3
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct StatusSnapshot {
    pub server: ServerMeta,
    pub tunnels: Vec<ServiceInfo>,
    pub agents: Vec<ServiceInfo>,
    pub channels: Vec<ServiceInfo>,
    pub pty_session_count: usize,
}

/// One row inside a `StatusSnapshot` category. Per-category extras
/// (provider for tunnels; crash_count/reason for channels; etc.) are
/// flattened in via `extra` — see each manager's build path for the
/// full set.
///
/// # Wire format (JSON)
/// ```json
/// {
///   "id": "feishu",
///   "name": "Feishu",
///   "status": { "state": "crashed" },
///   "uptime_secs": 42,
///   "reason": "plugin exited",
///   "crash_count": 2
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub status: ApiServiceStatus,
    pub uptime_secs: u64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Wire-level status across all service kinds (tunnels, agents, channels).
///
/// Unifies `ServiceStatus` (tunnels/agents) and `ChannelRunStatus`
/// (channel plugins) into one tagged enum. Serializes as a JSON object
/// with a `state` discriminant — consumers pattern-match on it.
///
/// # Wire format (JSON)
/// ```json
/// { "state": "running" }
/// { "state": "spawning" }
/// { "state": "not_started" }
/// { "state": "stopped", "reason": "killed" }      // reason may be null
/// { "state": "failed", "error": "spawn failed" }
/// { "state": "crashed" }
/// ```
///
/// Consumers (web, desktop-ui, future TUI/CLI) should define their own
/// schema at the wire boundary. The TS reference implementation lives
/// in `src/shared/client-ts/src/schemas.ts` (zod).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ApiServiceStatus {
    Running,
    Spawning,
    NotStarted,
    Stopped { reason: Option<String> },
    Failed { error: String },
    Crashed,
}

impl From<&ServiceStatus> for ApiServiceStatus {
    fn from(s: &ServiceStatus) -> Self {
        match s {
            ServiceStatus::Running => Self::Running,
            ServiceStatus::Stopped { reason } => Self::Stopped {
                reason: Some(reason.clone()),
            },
            ServiceStatus::Failed { error } => Self::Failed {
                error: error.clone(),
            },
        }
    }
}

pub(super) fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
