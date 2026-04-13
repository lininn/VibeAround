/**
 * Agents API: fetch enabled agents from backend.
 */

/** All dashboard routes live under /_va_/ to keep the root namespace free for
 *  cookie-based dev-server preview proxying. */
const VA_PREFIX = "/_va_";

function getBaseUrl(): string {
  if (typeof window === "undefined") return `http://127.0.0.1:12358${VA_PREFIX}`;
  return `${window.location.origin}${VA_PREFIX}`;
}

export interface AgentInfo {
  id: string;
  name: string;
  description: string;
}

export interface AgentsConfig {
  agents: AgentInfo[];
  default_agent: string;
}

export async function getAgents(): Promise<AgentsConfig> {
  const res = await fetch(`${getBaseUrl()}/api/agents`);
  if (!res.ok) throw new Error(`GET /api/agents: ${res.status}`);
  return res.json();
}
