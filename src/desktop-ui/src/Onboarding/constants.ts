export const ALL_AGENTS = ["claude", "opencode", "gemini", "codex"] as const;
export type AgentId = (typeof ALL_AGENTS)[number];

export const AGENT_LABELS: Record<AgentId, string> = {
  claude: "Claude Code",
  gemini: "Gemini CLI",
  opencode: "Opencode",
  codex: "Codex CLI",
};

export const TUNNEL_PROVIDERS = ["none", "localtunnel", "cloudflare", "ngrok"] as const;
export type TunnelProvider = (typeof TUNNEL_PROVIDERS)[number];

export const TUNNEL_LABELS: Record<TunnelProvider, string> = {
  none: "None (local only)",
  localtunnel: "Local Tunnel",
  cloudflare: "Cloudflare Tunnel",
  ngrok: "Ngrok",
};

export const STEPS = ["Welcome", "Agents", "Channels", "Tunnel", "Confirm"] as const;
export type OnboardingStep = (typeof STEPS)[number];
