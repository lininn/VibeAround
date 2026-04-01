import { Rocket } from "lucide-react";

import type { StepConfirmProps } from "../types";

export function StepConfirm({
  agents,
  tunnels,
  pluginRegistry,
  enabledAgents,
  defaultAgent,
  tunnelProvider,
  enabledChannels,
}: StepConfirmProps) {
  const agentLabels = new Map(agents.map((a) => [a.id, a.display_name]));
  const tunnelLabels = new Map(tunnels.map((t) => [t.id, t.display_name]));

  const agentSummary = Array.from(enabledAgents)
    .map((id) => `${agentLabels.get(id) ?? id}${id === defaultAgent ? " ★" : ""}`)
    .join(", ");

  const channelNames = Array.from(enabledChannels)
    .map((id) => {
      const registry = pluginRegistry.find((p) => p.id === id);
      return registry?.name ?? id;
    });

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-base font-semibold flex items-center gap-2">
          <Rocket className="w-4 h-4 text-primary" />
          Ready to Launch
        </h2>
        <p className="text-xs text-muted-foreground mt-1">
          Review your configuration. You can always change these in
          settings.json later.
        </p>
      </div>

      <div className="space-y-2 text-sm">
        <SummaryRow label="Agents" value={agentSummary} />
        <SummaryRow
          label="Channels"
          value={channelNames.length > 0 ? channelNames.join(", ") : "None configured"}
        />
        <SummaryRow label="Tunnel" value={tunnelLabels.get(tunnelProvider) ?? tunnelProvider} />
      </div>

      <p className="text-[11px] text-muted-foreground mt-3 leading-relaxed">
        VibeAround will add an MCP server entry to your coding agents' global
        settings and install a handover skill for session transfer between
        devices. Your existing agent settings will not be overwritten.
      </p>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start gap-3 py-2 px-3 rounded-md bg-muted/40">
      <span className="text-xs text-muted-foreground w-20 shrink-0 pt-0.5">
        {label}
      </span>
      <span className="text-sm">{value}</span>
    </div>
  );
}
