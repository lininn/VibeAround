"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from "./Conversation";
import { Message, MessageContent } from "./Message";
import { MessageResponse } from "./MessageResponse";
import { ChatInput } from "./ChatInput";

import { getWebSocketUrl } from "@/lib/ws-url";
import type { ToolType } from "@/lib/terminal-types";
import { toolThemes } from "@/lib/terminal-types";
import type { AgentInfo } from "@/api/agents";

function agentIdToToolType(id: string): ToolType {
  if (id in toolThemes) return id as ToolType;
  return "generic";
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export type ChatMessage = {
  role: "user" | "assistant" | "system";
  content: string;
  progress?: string;
};

export function ChatView() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const [streaming, setStreaming] = useState(false);

  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string>("claude");
  const [activeAgent, setActiveAgent] = useState<string>("claude");
  const wsRef = useRef<WebSocket | null>(null);
  const pendingAgentSwitchRef = useRef<{
    target: string;
    resolve: () => void;
    reject: (error: Error) => void;
  } | null>(null);

  const toolType = agentIdToToolType(selectedAgent);
  const agentLabel = capitalize(selectedAgent);

  useEffect(() => {
    const ws = new WebSocket(getWebSocketUrl("/ws/chat"));
    wsRef.current = ws;

    ws.onopen = () => setConnected(true);
    ws.onclose = () => {
      setConnected(false);
      setStreaming(false);
      if (pendingAgentSwitchRef.current) {
        pendingAgentSwitchRef.current.reject(new Error("Connection closed during agent switch."));
        pendingAgentSwitchRef.current = null;
      }
    };
    ws.onerror = () => setConnected(false);

    ws.onmessage = (event) => {
      if (typeof event.data !== "string") return;
      const s = event.data as string;

      let j: Record<string, unknown>;
      try {
        j = JSON.parse(s);
      } catch {
        appendToAssistant(s);
        return;
      }

      if (j.type === "config" && Array.isArray(j.agents)) {
        setAgents(j.agents as AgentInfo[]);
        if (typeof j.default_agent === "string") {
          setSelectedAgent(j.default_agent as string);
          setActiveAgent(j.default_agent as string);
        }
        return;
      }

      if (j.type === "agent_switched" && typeof j.agent === "string") {
        const agent = j.agent as string;
        setActiveAgent(agent);
        if (pendingAgentSwitchRef.current?.target === agent) {
          pendingAgentSwitchRef.current.resolve();
          pendingAgentSwitchRef.current = null;
        }
        return;
      }

      if (j.type === "system_text" && typeof j.text === "string") {
        const text = j.text as string;
        const switchedMatch = /^Switched agent to\s+(.+?)\.?$/i.exec(text.trim());
        if (switchedMatch) {
          const switchedAgent = switchedMatch[1].trim().toLowerCase();
          setActiveAgent(switchedAgent);
          if (pendingAgentSwitchRef.current?.target === switchedAgent) {
            pendingAgentSwitchRef.current.resolve();
            pendingAgentSwitchRef.current = null;
          }
          return;
        }

        if (pendingAgentSwitchRef.current && /^Unknown agent:|^Agent is disabled:/i.test(text.trim())) {
          pendingAgentSwitchRef.current.reject(new Error(text));
          pendingAgentSwitchRef.current = null;
          setStreaming(false);
          setMessages((prev) => {
            const next = [...prev];
            const last = next[next.length - 1];
            if (last?.role === "assistant" && last.content === "") {
              next.pop();
            }
            return next;
          });
          return;
        }

        setMessages((prev) => [...prev, { role: "system", content: text }]);
        setStreaming(false);
        return;
      }

      if (j.done === true) {
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last?.role === "assistant" && last.progress) {
            const next = [...prev];
            next[next.length - 1] = { ...last, progress: undefined };
            return next;
          }
          return prev;
        });
        setStreaming(false);
        return;
      }

      if (typeof j.error === "string") {
        if (pendingAgentSwitchRef.current) {
          pendingAgentSwitchRef.current.reject(new Error(j.error as string));
          pendingAgentSwitchRef.current = null;
        }
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last?.role === "assistant") {
            const next = [...prev];
            next[next.length - 1] = {
              ...last,
              content: last.content + (last.content ? "\n\n" : "") + `Error: ${j.error}`,
              progress: undefined,
            };
            return next;
          }
          return [...prev, { role: "assistant", content: `Error: ${j.error}` }];
        });
        setStreaming(false);
        return;
      }

      if (typeof j.progress === "string") {
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last?.role === "assistant") {
            const next = [...prev];
            next[next.length - 1] = { ...last, progress: j.progress as string };
            return next;
          }
          return prev;
        });
        return;
      }

      if (typeof j.text === "string") {
        appendToAssistant(j.text as string);
        return;
      }
    };

    function appendToAssistant(text: string) {
      if (!text) return;
      setMessages((prev) => {
        if (prev.length === 0) return [{ role: "assistant", content: text }];
        const last = prev[prev.length - 1];
        if (last.role !== "assistant") {
          return [...prev, { role: "assistant", content: text }];
        }
        const next = [...prev];
        next[next.length - 1] = { ...last, content: last.content + text, progress: undefined };
        return next;
      });
    }

    return () => {
      ws.close();
      wsRef.current = null;
    };
  }, []);

  const switchAgentIfNeeded = useCallback(async () => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket is not connected.");
    }
    if (selectedAgent === activeAgent) return;

    if (pendingAgentSwitchRef.current) {
      throw new Error("Agent switch already in progress.");
    }

    await new Promise<void>((resolve, reject) => {
      pendingAgentSwitchRef.current = { target: selectedAgent, resolve, reject };
      ws.send(JSON.stringify({ type: "message", text: `/agent ${selectedAgent}` }));
    });
  }, [activeAgent, selectedAgent]);

  const sendMessage = useCallback(async () => {
    const text = input.trim();
    if (!text || !wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;

    setInput("");
    setMessages((prev) => [
      ...prev,
      { role: "user", content: text },
      { role: "assistant", content: "" },
    ]);
    setStreaming(true);

    try {
      await switchAgentIfNeeded();
      wsRef.current.send(JSON.stringify({ type: "message", text }));
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to switch agent.";
      setMessages((prev) => {
        const next = [...prev];
        const last = next[next.length - 1];
        if (last?.role === "assistant") {
          next[next.length - 1] = {
            ...last,
            content: last.content + (last.content ? "\n\n" : "") + `Error: ${message}`,
            progress: undefined,
          };
          return next;
        }
        return [...prev, { role: "assistant", content: `Error: ${message}` }];
      });
      setStreaming(false);
    }
  }, [input, switchAgentIfNeeded]);

  const handleAgentChange = useCallback((agentId: string) => {
    setSelectedAgent(agentId);
  }, []);

  return (
    <div className="flex h-full flex-col overflow-hidden bg-background">
      <Conversation className="flex-1">
        <ConversationContent>
          {messages.length === 0 ? (
            <ConversationEmptyState
              title={`Chat with ${agentLabel}`}
              description="Send a message to start."
            />
          ) : (
            messages.map((msg, i) => (
              <Message key={i} from={msg.role}>
                <MessageContent
                  className={
                    msg.role === "user"
                      ? "rounded-lg bg-primary/15 px-4 py-3 text-foreground"
                      : msg.role === "system"
                        ? "rounded-lg border border-border/60 bg-muted/20 px-4 py-3 text-muted-foreground"
                        : "rounded-lg bg-muted/50 px-4 py-3 text-foreground"
                  }
                >
                  {msg.role === "user" ? (
                    <p className="whitespace-pre-wrap text-sm">{msg.content}</p>
                  ) : msg.role === "system" ? (
                    <p className="whitespace-pre-wrap text-xs font-mono leading-5">{msg.content}</p>
                  ) : (
                    <>
                      <MessageResponse
                        content={msg.content}
                        isStreaming={streaming && i === messages.length - 1}
                      />
                      {msg.progress && (
                        <span className="text-xs text-muted-foreground/60 font-mono animate-pulse">
                          {msg.progress}
                        </span>
                      )}
                    </>
                  )}
                </MessageContent>
              </Message>
            ))
          )}
        </ConversationContent>
        <ConversationScrollButton />
      </Conversation>

      <ChatInput
        value={input}
        onChange={setInput}
        onSubmit={() => {
          void sendMessage();
        }}
        disabled={!connected}
        isStreaming={streaming}
        placeholder={connected ? `Message ${agentLabel}…` : "Connecting…"}
        targetLabel={agentLabel}
        targetTool={toolType}
        agents={agents}
        onAgentChange={handleAgentChange}
      />
    </div>
  );
}
