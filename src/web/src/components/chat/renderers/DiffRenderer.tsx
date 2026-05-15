"use client";

import { FileDiff } from "lucide-react";
import { lineCount } from "./contentUtils";
import type { ToolCallContent } from "@agentclientprotocol/sdk";

type DiffContent = Extract<ToolCallContent, { type: "diff" }>;

export function DiffRenderer({ diff }: { diff: DiffContent }) {
  return (
    <details className="rounded-md border border-border/70 bg-background/60 px-3 py-2">
      <summary className="flex cursor-pointer items-center gap-2 text-sm">
        <FileDiff className="h-4 w-4 text-primary" />
        <span className="min-w-0 truncate font-medium">{diff.path}</span>
        <span className="ml-auto shrink-0 font-mono text-xs text-muted-foreground">
          {lineCount(diff.oldText)} {"\u2192"} {lineCount(diff.newText)}
        </span>
      </summary>
      <div className="mt-3 grid gap-3 md:grid-cols-2">
        {diff.oldText !== null && diff.oldText !== undefined && (
          <pre className="max-h-72 overflow-auto whitespace-pre-wrap rounded bg-muted/35 p-3 text-xs leading-5 text-muted-foreground">
            {diff.oldText}
          </pre>
        )}
        <pre className="max-h-72 overflow-auto whitespace-pre-wrap rounded bg-muted/35 p-3 text-xs leading-5 text-muted-foreground">
          {diff.newText}
        </pre>
      </div>
    </details>
  );
}
