"use client";

import { Code2 } from "lucide-react";
import { useI18n } from "@va/i18n";
import { cn } from "@/lib/utils";
import type { ChatPlanPart } from "../chatTypes";

export function PlanRenderer({ part }: { part: ChatPlanPart }) {
  const { t } = useI18n();

  if (part.plan.entries.length === 0) return null;

  return (
    <div className="rounded-md border border-border/70 bg-muted/20 px-3 py-3">
      <div className="mb-2 flex items-center gap-2 text-xs font-medium uppercase text-muted-foreground">
        <Code2 className="h-3.5 w-3.5" />
        {t("Plan")}
      </div>
      <div className="space-y-2">
        {part.plan.entries.map((entry, index) => (
          <div key={`${entry.content}-${index}`} className="flex min-w-0 items-start gap-2">
            <span
              className={cn(
                "mt-1 h-2 w-2 shrink-0 rounded-full",
                entry.status === "completed"
                  ? "bg-primary/70"
                  : entry.status === "in_progress"
                    ? "bg-amber-400"
                    : "bg-muted-foreground/35",
              )}
            />
            <div className="min-w-0 flex-1">
              <div className="text-sm leading-5 text-foreground">{entry.content}</div>
              <div className="mt-0.5 font-mono text-[10px] uppercase text-muted-foreground">
                {entry.status} · {entry.priority}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
