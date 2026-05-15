export type ChatActivity = {
  id: string;
  kind: "thinking" | "tool";
  label: string;
  detail?: string;
  status?: string;
  active?: boolean;
};

export type ChatMessage = {
  role: "user" | "assistant";
  content: string;
  progress?: string;
  activities?: ChatActivity[];
  mode?: "standalone" | "stream";
};

export type ChatMeta = {
  channelId?: string;
  sessionId?: string;
  agentTitle?: string;
  agentVersion?: string;
  agentName?: string;
};

export type ChatSessionSelection =
  | { kind: "current" }
  | { kind: "new" }
  | { kind: "resume"; sessionId: string };

export type PendingPermission = {
  requestId: string;
  request: unknown;
};
