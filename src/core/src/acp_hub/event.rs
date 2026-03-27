//! System events emitted by ACPHub for lifecycle observability.
//!
//! ACP protocol events (streaming tokens, tool calls) flow through the
//! BridgeClientHandler chain and do NOT appear here.

use crate::acp::routing::RouteKey;

use agent_client_protocol as acp;

use super::pod::PodSnapshot;

#[derive(Debug, Clone)]
pub enum SystemEvent {
    RouteCreated {
        route: RouteKey,
    },
    RouteClosed {
        route: RouteKey,
        reason: Option<String>,
    },
    RouteFailed {
        route: RouteKey,
        error: String,
    },
    AgentInitialized {
        route: RouteKey,
        cli_kind: Option<String>,
        profile: Option<String>,
        initialize: acp::InitializeResponse,
    },
    AgentInitializeFailed {
        route: RouteKey,
        cli_kind: Option<String>,
        error: String,
    },
    SessionReady {
        route: RouteKey,
        session_id: String,
    },
    SnapshotChanged {
        route: RouteKey,
        snapshot: PodSnapshot,
    },
}

impl SystemEvent {
    pub fn route(&self) -> &RouteKey {
        match self {
            Self::RouteCreated { route }
            | Self::RouteClosed { route, .. }
            | Self::RouteFailed { route, .. }
            | Self::AgentInitialized { route, .. }
            | Self::AgentInitializeFailed { route, .. }
            | Self::SessionReady { route, .. }
            | Self::SnapshotChanged { route, .. } => route,
        }
    }
}
