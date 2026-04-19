//! Shared runtime status primitives.
//!
//! Each domain manager (`ChannelMonitor`, `ACPHub`, `TunnelManager`,
//! `PtyRegistry`) owns its own state and exposes it through
//! [`StateSource`]; this module only holds the small common pieces they
//! share:
//!
//! - [`ServiceStatus`] — internal status enum for tunnel/agent entries.
//! - [`ServiceMeta`]   — runtime meta (status + started_at + abort).
//! - [`ApiServiceStatus`] — tagged wire enum reused by `TunnelRuntime`.
//!
//! [`StateSource`]: crate::state::StateSource

mod snapshot;
mod status;

pub use snapshot::ApiServiceStatus;
pub use status::{ServiceMeta, ServiceStatus};
