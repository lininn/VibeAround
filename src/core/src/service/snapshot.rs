//! `ApiServiceStatus` + `ServerMeta` — shared wire-adjacent types.
//!
//! Most of this file used to define a `StatusSnapshot` aggregate
//! returned by the legacy `/api/services` endpoint. That endpoint and
//! its aggregate type were removed in Phase 1g; per-domain endpoints
//! (`/api/channels`, `/api/tunnels`, `/api/agents/runtime`) each
//! define their own wire shapes in `src/server/src/api_types.rs`.
//!
//! `ApiServiceStatus` stays because it's still the natural wire shape
//! for "how is this one service doing" — reused by `TunnelRuntime`.
//! `ServerMeta` stays because the daemon entry still carries it.

use serde::Serialize;

use super::status::ServiceStatus;

/// Daemon metadata captured at start. Not currently exposed on the
/// wire but kept on `ServiceStatusManager` for possible future reuse
/// (e.g. a per-domain `/api/server/meta` endpoint).
///
/// # Wire format (JSON, if serialized)
/// ```json
/// { "started_at": 1713456789, "port": 12358 }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ServerMeta {
    pub started_at: u64,
    pub port: u16,
}

/// Wire-level status across service kinds. Serializes as a tagged
/// object with a `state` discriminant so consumers pattern-match
/// exhaustively instead of reverse-parsing free-form strings.
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
/// Reference zod schema:
/// `src/shared/client-ts/src/schemas.ts::ApiServiceStatusSchema`.
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
