"use client";

import { useI18n } from "@va/i18n";
import type { ChatThoughtPart } from "../chatTypes";

export function ThoughtRenderer({ part }: { part: ChatThoughtPart }) {
  const { t } = useI18n();
  const text = part.blocks
    .map((block) => (block.type === "text" ? block.text : ""))
    .join("");

  if (!text.trim()) return null;

  return (
    <details className="rounded-md border border-border/60 bg-muted/15 px-3 py-2 text-muted-foreground">
      <summary className="cursor-pointer font-mono text-xs uppercase">
        {t("Thinking")}
      </summary>
      <p className="mt-2 whitespace-pre-wrap text-xs leading-5">{text}</p>
    </details>
  );
}
