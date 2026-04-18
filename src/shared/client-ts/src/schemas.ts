/**
 * zod schemas for everything the dashboard server returns over HTTP
 * or WebSocket.
 *
 * The source of truth is Rust: look at the `#[derive(Serialize)]` types
 * in `src/core/src/service/snapshot.rs` and `src/server/src/api_types.rs`
 * for the reference shapes. The docstrings on those Rust types carry
 * JSON examples; this file mirrors them. When the Rust side changes,
 * update the matching schema here in the same PR.
 *
 * Usage: call `.parse()` on every wire-crossing value so bad payloads
 * fail fast at the boundary instead of rotting through the UI.
 */

import { z } from "zod";

// ---------------------------------------------------------------------------
// Agent IDs (mirrors resources/agents.json — order not significant)
// ---------------------------------------------------------------------------

/** Every agent ID defined in `resources/agents.json`. Hand-maintained.
 *  When that file adds an entry, add it here too and the `Record<AgentId, ...>`
 *  consumers (display-name maps) will force you to supply the rest. */
export const AGENT_IDS = [
  "claude",
  "gemini",
  "opencode",
  "codex",
  "cursor",
  "kiro",
  "qwen-code",
] as const;

export type AgentId = (typeof AGENT_IDS)[number];

export const AgentIdSchema = z.enum(AGENT_IDS);

// ---------------------------------------------------------------------------
// Constants mirrored from Rust
// ---------------------------------------------------------------------------

/** Mirror of `common::preview_entries::SHARE_TTL_SECS`. */
export const PREVIEW_SHARE_TTL_SECS = 600;

// ---------------------------------------------------------------------------
// GET /api/agents — enabled agent list + default
// ---------------------------------------------------------------------------

export const AgentInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
});
export type AgentInfo = z.infer<typeof AgentInfoSchema>;

export const AgentsConfigSchema = z.object({
  agents: z.array(AgentInfoSchema),
  default_agent: z.string(),
});
export type AgentsConfig = z.infer<typeof AgentsConfigSchema>;

// ---------------------------------------------------------------------------
// GET /api/services — unified runtime snapshot (transitional — Phase 1f
// splits this into per-manager endpoints)
// ---------------------------------------------------------------------------

export const ApiServiceStatusSchema = z.discriminatedUnion("state", [
  z.object({ state: z.literal("running") }),
  z.object({ state: z.literal("spawning") }),
  z.object({ state: z.literal("not_started") }),
  z.object({ state: z.literal("stopped"), reason: z.string().nullable() }),
  z.object({ state: z.literal("failed"), error: z.string() }),
  z.object({ state: z.literal("crashed") }),
]);
export type ApiServiceStatus = z.infer<typeof ApiServiceStatusSchema>;

/** ServiceInfo rows carry per-category extras via `#[serde(flatten)]` on
 *  the Rust side (see the `#[serde(flatten)] pub extra` field on
 *  `common::service::snapshot::ServiceInfo`). Known extras are declared
 *  here as optional so consumers get autocomplete; `passthrough()`
 *  lets any additional keys come through as `unknown`. */
export const ServiceInfoSchema = z
  .object({
    id: z.string(),
    name: z.string(),
    status: ApiServiceStatusSchema,
    uptime_secs: z.number(),
    // Tunnel extras
    provider: z.string().optional(),
    url: z.string().optional(),
    // Channel extras (from ChannelMonitor snapshot)
    reason: z.string().optional(),
    crash_count: z.number().optional(),
    last_seen_age_secs: z.number().optional(),
    restart_in_secs: z.number().optional(),
    // Agent runtime extras
    kind: z.string().optional(),
    workspace: z.string().optional(),
    role: z.enum(["manager", "worker"]).optional(),
  })
  .passthrough();
export type ServiceInfo = z.infer<typeof ServiceInfoSchema>;

export const ServerMetaSchema = z.object({
  started_at: z.number(),
  port: z.number(),
});
export type ServerMeta = z.infer<typeof ServerMetaSchema>;

export const StatusSnapshotSchema = z.object({
  server: ServerMetaSchema,
  tunnels: z.array(ServiceInfoSchema),
  agents: z.array(ServiceInfoSchema),
  channels: z.array(ServiceInfoSchema),
  pty_session_count: z.number(),
});
export type StatusSnapshot = z.infer<typeof StatusSnapshotSchema>;
