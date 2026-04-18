/**
 * Agents API: fetch enabled agents from backend.
 */

import { browserBaseUrl, AgentsConfigSchema, type AgentInfo, type AgentsConfig } from "@va/client";

export type { AgentInfo, AgentsConfig };

export async function getAgents(): Promise<AgentsConfig> {
  const res = await fetch(`${browserBaseUrl()}/api/agents`);
  if (!res.ok) throw new Error(`GET /api/agents: ${res.status}`);
  return AgentsConfigSchema.parse(await res.json());
}
