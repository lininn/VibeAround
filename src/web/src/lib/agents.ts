import type { ToolType } from "@/lib/terminal-types";

export interface AgentDisplayInfo {
  id: string;
  name: string;
}

export const AGENT_DISPLAY_NAMES: Record<string, string> = {
  claude: "Claude Code",
  gemini: "Gemini CLI",
  codex: "Codex CLI",
  opencode: "Opencode",
};

export function getAgentDisplayName(agentId: string): string {
  return AGENT_DISPLAY_NAMES[agentId] ?? agentId;
}

export function getToolDisplayName(tool: string): string {
  return AGENT_DISPLAY_NAMES[tool.toLowerCase()] ?? "Terminal";
}

export function agentIdToToolType(agentId: string): ToolType {
  const normalized = agentId.toLowerCase();
  if (normalized === "claude" || normalized === "codex" || normalized === "gemini" || normalized === "opencode") {
    return normalized;
  }
  return "generic";
}
