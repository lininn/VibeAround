//! `ServiceStatusManager`: thin holder for cross-domain runtime state.
//!
//! Phase 1g has progressively dismantled this module. What used to be an
//! aggregating facade (StatusSnapshot over tunnels + channels + agents +
//! PTY) is now just a container for:
//!
//! - `tunnels` — owned `TunnelManager` instance (with its own
//!   `StateSource` + `subscribe_changes`). Exposed via `tunnels()`.
//! - `channel_monitor` — `Weak` back-ref set once at daemon boot, used by
//!   stop/restart/start handlers to reach the real supervisor.
//! - `pty` — shared PTY session registry (keyed by session UUID).
//! - `server_meta` + `port` — boot-time server identity.
//!
//! Per-domain HTTP/WS endpoints (`/api/channels`, `/api/tunnels`,
//! `/api/agents/runtime`) read each manager directly via the
//! `StateSource` trait — they do not go through this struct.

mod entries;
mod snapshot;
mod status;

use std::sync::{Arc, Weak};

use parking_lot::RwLock;
use tokio::task::AbortHandle;

use crate::channel_manager::monitor::ChannelMonitor;

use crate::pty::{unix_now_secs, Registry};
use crate::tunnels::{TunnelManager, TunnelProvider};

pub use entries::{AgentStatusEntry, ChannelEntry};
pub use snapshot::{ApiServiceStatus, ServerMeta};
pub use status::{spawn_tracked, ServiceMeta, ServiceStatus};

// ---------------------------------------------------------------------------
// ServiceStatusManager
// ---------------------------------------------------------------------------

pub struct ServiceStatusManager {
    /// `ChannelMonitor` back-ref (Weak to avoid cycle with `ChannelManager`).
    /// Set once at daemon boot via `set_channel_monitor`. Handlers use this
    /// to reach `force_stop` / `force_start` etc.
    channel_monitor: RwLock<Weak<ChannelMonitor>>,
    /// Tunnel registry (at most one per provider in normal operation).
    /// Owned directly; callers that need live updates subscribe via
    /// `tunnels().subscribe_changes()`.
    tunnels: Arc<TunnelManager>,
    /// PTY sessions (reuses existing `Registry`).
    pub pty: Registry,
    /// Web server metadata.
    pub server_meta: ServerMeta,
    /// Convenience: the port the web server listens on.
    pub port: u16,
}

impl ServiceStatusManager {
    pub fn new(port: u16) -> Self {
        Self {
            channel_monitor: RwLock::new(Weak::new()),
            tunnels: TunnelManager::new(),
            pty: Arc::new(dashmap::DashMap::new()),
            server_meta: ServerMeta {
                started_at: unix_now_secs(),
                port,
            },
            port,
        }
    }

    // -----------------------------------------------------------------------
    // Channel monitor (set once at daemon boot)
    // -----------------------------------------------------------------------

    pub fn set_channel_monitor(&self, monitor: Weak<ChannelMonitor>) {
        *self.channel_monitor.write() = monitor;
    }

    pub fn channel_monitor(&self) -> Option<Arc<ChannelMonitor>> {
        self.channel_monitor.read().upgrade()
    }

    /// Clear all service entries. Called on daemon stop to prevent stale
    /// entries from persisting across restarts.
    pub fn clear(&self) {
        self.tunnels.clear();
        self.pty.clear();
    }

    /// Shared `TunnelManager` — the canonical way to observe or
    /// mutate tunnel state. The thin facade methods below
    /// (`register_tunnel`, `set_tunnel_url`, `has_tunnel_url`,
    /// `get_tunnel_url`) are kept so existing callers don't have to
    /// reach through `tunnels()` for every operation.
    pub fn tunnels(&self) -> Arc<TunnelManager> {
        Arc::clone(&self.tunnels)
    }

    // -----------------------------------------------------------------------
    // Tunnel facade (thin delegates to TunnelManager)
    // -----------------------------------------------------------------------

    pub fn register_tunnel(&self, provider: TunnelProvider, abort_handle: AbortHandle) {
        self.tunnels.register(provider, abort_handle);
    }

    pub fn set_tunnel_url(&self, provider_key: &str, url: &str) {
        self.tunnels.set_url(provider_key, url);
    }

    pub fn has_tunnel_url(&self) -> bool {
        self.tunnels.has_url()
    }

    pub fn get_tunnel_url(&self) -> Option<String> {
        self.tunnels.first_url()
    }

}
