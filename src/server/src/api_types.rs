//! HTTP/WebSocket API response shapes for the dashboard.
//!
//! This module owns the **wire contract** between the server and its
//! frontends (web dashboard, Tauri desktop-ui, plus any future TUI / CLI
//! / third-party consumer). Types here exist only to be serialized.
//!
//! # Where the data comes from
//!
//! Structs in this module are populated by reading `common` core state
//! (via `config::ensure_loaded()` and `resources::...`). The core does
//! not know about HTTP; it exposes domain data and this module maps it
//! to wire shapes. Consumers that aren't HTTP (TUI, CLI) should write
//! their own mapping alongside core, not reuse these types.
//!
//! # Consumers
//!
//! The canonical TS validator/types live in
//! `src/shared/client-ts/src/schemas.ts` (zod). Keep the wire shapes
//! documented on each struct below so Python/Swift/curl consumers can
//! derive their own schemas without reading the zod file.

use serde::Serialize;

/// Per-agent display info returned under `AgentsConfig.agents`.
///
/// # Wire format (JSON)
/// ```json
/// { "id": "claude", "name": "Claude Code", "description": "Claude Code CLI" }
/// ```
///
/// - `id`: an agent ID from `resources/agents.json` (e.g. `"claude"`,
///   `"gemini"`, `"qwen-code"`).
/// - `name` / `description`: copied from that file's `display_name` and
///   `description` fields.
#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// `GET /api/agents` response envelope.
///
/// # Wire format (JSON)
/// ```json
/// {
///   "agents": [
///     { "id": "claude", "name": "Claude Code", "description": "..." },
///     { "id": "gemini", "name": "Gemini CLI",  "description": "..." }
///   ],
///   "default_agent": "claude"
/// }
/// ```
///
/// - `agents`: the enabled subset from settings.json (not all agents in
///   `agents.json`), ordered as configured.
/// - `default_agent`: raw string from settings.json. The server does not
///   cross-validate against `agents` — consumers should treat an
///   unrecognized value as "no default".
#[derive(Debug, Clone, Serialize)]
pub struct AgentsConfig {
    pub agents: Vec<AgentInfo>,
    pub default_agent: String,
}

impl AgentInfo {
    /// Build an `AgentInfo` for each of the given agent IDs by looking up
    /// the corresponding entry in `agents.json`. IDs with no matching
    /// entry are silently dropped.
    pub fn for_ids(ids: &[String]) -> Vec<Self> {
        ids.iter()
            .filter_map(|id| {
                let def = common::resources::agent_by_id(id)?;
                Some(Self {
                    id: id.clone(),
                    name: def.display_name.clone(),
                    description: def.description.clone(),
                })
            })
            .collect()
    }
}
