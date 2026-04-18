import type { AgentKind } from "@va/generated/AgentKind";
import type { ToolType } from "@/lib/terminal-types";

export type { AgentKind };

export interface AgentDisplayInfo {
  id: AgentKind;
  name: string;
}

/** Display names for every `AgentKind`. The `Record<AgentKind, ...>` shape
 *  means adding a new variant in Rust breaks the build here until the name
 *  is filled in. */
export const AGENT_DISPLAY_NAMES: Record<AgentKind, string> = {
  claude: "Claude Code",
  gemini: "Gemini CLI",
  opencode: "Opencode",
  codex: "Codex CLI",
  cursor: "Cursor",
  kiro: "Kiro",
  "qwen-code": "Qwen Code",
};

export const AGENT_KINDS = Object.keys(AGENT_DISPLAY_NAMES) as readonly AgentKind[];

function isAgentKind(value: string): value is AgentKind {
  return value in AGENT_DISPLAY_NAMES;
}

export function getAgentDisplayName(agentId: string): string {
  return isAgentKind(agentId) ? AGENT_DISPLAY_NAMES[agentId] : agentId;
}

export function getToolDisplayName(tool: string): string {
  const normalized = tool.toLowerCase();
  return isAgentKind(normalized) ? AGENT_DISPLAY_NAMES[normalized] : "Terminal";
}

export function agentIdToToolType(agentId: string): ToolType {
  const normalized = agentId.toLowerCase();
  // ToolType is a subset of AgentKind (plus "generic"). Widen here and let
  // the theme layer handle any kind it doesn't recognize as "generic".
  if (normalized === "claude" || normalized === "codex" || normalized === "gemini" || normalized === "opencode") {
    return normalized;
  }
  return "generic";
}
