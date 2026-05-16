"use client";

import { ContentBlockRenderer } from "./ContentBlockRenderer";
import { DiffRenderer } from "./DiffRenderer";
import { TerminalReferenceRenderer } from "./TerminalReferenceRenderer";
import type { ToolCallContent } from "@agentclientprotocol/sdk";

export function ToolContentRenderer({ item }: { item: ToolCallContent }) {
  switch (item.type) {
    case "content":
      return <ContentBlockRenderer block={item.content} role="assistant" />;
    case "diff":
      return <DiffRenderer diff={item} />;
    case "terminal":
      return <TerminalReferenceRenderer terminal={item} />;
  }
}
