"use client";

import { cjk } from "@streamdown/cjk";
import { code } from "@streamdown/code";
import { Streamdown } from "streamdown";

interface MarkdownRendererProps {
  content: string;
  isStreaming?: boolean;
  className?: string;
}

export function MarkdownRenderer({
  content,
  isStreaming = false,
  className,
}: MarkdownRendererProps) {
  return (
    <Streamdown
      className={[
        "prose prose-sm dark:prose-invert max-w-none text-sm",
        "[&>*:first-child]:mt-0 [&>*:last-child]:mb-0",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
      plugins={{ cjk, code }}
      shikiTheme={["github-light", "github-dark"]}
      isAnimating={isStreaming}
      parseIncompleteMarkdown={true}
    >
      {content}
    </Streamdown>
  );
}
