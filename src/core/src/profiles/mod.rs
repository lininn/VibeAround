//! Shared profile runtime.
//!
//! Profiles are user-managed provider credentials plus the catalog metadata
//! needed to render env vars and profile-local config files for coding CLIs.
//! Desktop owns the UI and terminal window launch; core owns the reusable
//! schema/catalog/rendering path so IM-started agents can use the same
//! profiles.

pub mod catalog;
pub mod render;
pub mod runtime;
pub mod schema;

pub use schema::{AuthMode, ProfileDef};

pub fn normalize_legacy_profile(mut profile: ProfileDef) -> ProfileDef {
    // Azure used to have only one API kind in early catalog iterations.
    // Profiles saved during that window should inherit endpoint/deployment
    // values across both kinds so users can keep editing without retyping.
    if profile.provider == "azure"
        && profile.api_types.iter().any(|t| t == "openai-responses")
        && !profile.api_types.iter().any(|t| t == "openai-chat")
    {
        profile.api_types.push("openai-chat".to_string());
        if let Some(overrides) = profile.overrides.get("openai-responses").cloned() {
            profile
                .overrides
                .entry("openai-chat".to_string())
                .or_insert(overrides);
        }
    }
    profile
}
