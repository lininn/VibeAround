//! Workspace domain model.
//!
//! A workspace owns threads. IM/Web routes attach to workspace threads; agent
//! sessions remain implementation details of each thread runtime.

pub mod manager;
pub mod registry;
pub mod store;
pub mod threads;

pub use manager::WorkspaceThreadManager;
pub use registry::{WorkspaceId, WorkspaceProjection, WorkspaceRecord, GENERAL_WORKSPACE_ID};
