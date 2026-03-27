//! Session manager: future session persistence layer.
//!
//! Currently a skeleton that projects SystemEvents into SessionManagerEvents.
//! Will eventually handle session save/load/list/delete.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::acp_hub::event::SystemEvent;
use crate::acp::routing::RouteKey;

#[derive(Debug, Clone)]
pub enum SessionManagerEvent {
    SessionReady { route: RouteKey, session_id: String },
    SessionClosed { route: RouteKey },
}

pub struct SessionManager {
    event_tx: broadcast::Sender<SessionManagerEvent>,
}

impl SessionManager {
    pub fn new() -> Arc<Self> {
        let (event_tx, _) = broadcast::channel(32);
        Arc::new(Self { event_tx })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SessionManagerEvent> {
        self.event_tx.subscribe()
    }

    pub fn project_event(&self, event: &SystemEvent) {
        match event {
            SystemEvent::SessionReady { route, session_id } => {
                let _ = self.event_tx.send(SessionManagerEvent::SessionReady {
                    route: route.clone(),
                    session_id: session_id.clone(),
                });
            }
            SystemEvent::RouteClosed { route, .. }
            | SystemEvent::RouteFailed { route, .. } => {
                let _ = self.event_tx.send(SessionManagerEvent::SessionClosed {
                    route: route.clone(),
                });
            }
            _ => {}
        }
    }
}
