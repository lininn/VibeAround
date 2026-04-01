import { Bot, Check } from "lucide-react";

import type { StepAgentsProps } from "../types";

export function StepAgents({
  agents,
  enabled,
  defaultAgent,
  onToggle,
  onSetDefault,
}: StepAgentsProps) {
  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-base font-semibold flex items-center gap-2">
          <Bot className="w-4 h-4 text-primary" />
          Agents
        </h2>
        <p className="text-xs text-muted-foreground mt-1">
          Choose which AI coding agents to enable. At least one is required.
        </p>
      </div>
      <div className="grid grid-cols-2 gap-2">
        {agents.map((agent) => {
          const isEnabled = enabled.has(agent.id);
          const isDefault = defaultAgent === agent.id;
          return (
            <div
              key={agent.id}
              className={`relative flex flex-col gap-1.5 p-3 rounded-lg border cursor-pointer transition-colors ${
                isEnabled
                  ? "border-primary/40 bg-primary/5"
                  : "border-border hover:border-border/80"
              }`}
              onClick={() => onToggle(agent.id)}
            >
              <div className="flex items-center justify-between">
                <span
                  className={`text-sm font-medium ${
                    isEnabled ? "text-foreground" : "text-muted-foreground"
                  }`}
                >
                  {agent.display_name}
                </span>
                <div
                  className={`w-4 h-4 rounded border flex items-center justify-center transition-colors ${
                    isEnabled
                      ? "bg-primary border-primary"
                      : "border-muted-foreground/30"
                  }`}
                >
                  {isEnabled && (
                    <Check className="w-3 h-3 text-primary-foreground" />
                  )}
                </div>
              </div>
              {isEnabled && (
                <button
                  onClick={(event) => {
                    event.stopPropagation();
                    onSetDefault(agent.id);
                  }}
                  className={`text-[10px] font-mono px-1.5 py-0.5 rounded self-start transition-colors ${
                    isDefault
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted text-muted-foreground hover:bg-accent"
                  }`}
                >
                  {isDefault ? "★ default" : "set default"}
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
