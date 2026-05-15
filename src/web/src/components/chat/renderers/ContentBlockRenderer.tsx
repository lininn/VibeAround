"use client";

import {
  FileAudio,
  FileText,
  Image as ImageIcon,
  Link,
} from "lucide-react";
import { useI18n } from "@va/i18n";
import { MessageResponse } from "../MessageResponse";
import { dataUrl, fileNameFromUri } from "./contentUtils";
import type { ChatMessage } from "../chatTypes";
import type { ContentBlock } from "@agentclientprotocol/sdk";

interface ContentBlockRendererProps {
  block: ContentBlock;
  role: ChatMessage["role"];
  isStreaming?: boolean;
}

export function ContentBlockRenderer({
  block,
  role,
  isStreaming,
}: ContentBlockRendererProps) {
  const { t } = useI18n();

  switch (block.type) {
    case "text":
      return role === "user" ? (
        <p className="whitespace-pre-wrap text-sm leading-6">{block.text}</p>
      ) : (
        <MessageResponse content={block.text} isStreaming={isStreaming} />
      );
    case "image":
      return (
        <figure className="overflow-hidden rounded-md border border-border/70 bg-muted/20">
          <img
            src={block.uri ?? dataUrl(block.mimeType, block.data)}
            alt={block.uri ? fileNameFromUri(block.uri) : t("Image")}
            className="max-h-[28rem] w-full object-contain"
            loading="lazy"
          />
          <figcaption className="flex items-center gap-2 border-t border-border/60 px-3 py-2 text-xs text-muted-foreground">
            <ImageIcon className="h-3.5 w-3.5" />
            <span className="truncate">{block.uri ?? block.mimeType}</span>
          </figcaption>
        </figure>
      );
    case "audio":
      return (
        <div className="rounded-md border border-border/70 bg-muted/20 px-3 py-3">
          <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
            <FileAudio className="h-3.5 w-3.5" />
            <span>{block.mimeType}</span>
          </div>
          <audio controls src={dataUrl(block.mimeType, block.data)} className="w-full" />
        </div>
      );
    case "resource_link":
      return (
        <a
          href={block.uri}
          target="_blank"
          rel="noreferrer"
          className="flex min-w-0 items-start gap-3 rounded-md border border-border/70 bg-muted/20 px-3 py-2 text-sm hover:bg-muted/35"
        >
          <Link className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0">
            <span className="block truncate font-medium text-foreground">
              {block.title ?? block.name}
            </span>
            <span className="block truncate text-xs text-muted-foreground">{block.uri}</span>
            {block.description && (
              <span className="mt-1 block text-xs text-muted-foreground/80">
                {block.description}
              </span>
            )}
          </span>
        </a>
      );
    case "resource": {
      const resource = block.resource;
      const label = fileNameFromUri(resource.uri);
      if ("text" in resource) {
        return (
          <details className="rounded-md border border-border/70 bg-muted/20 px-3 py-2">
            <summary className="flex cursor-pointer items-center gap-2 text-sm font-medium text-foreground">
              <FileText className="h-4 w-4 text-muted-foreground" />
              <span className="min-w-0 truncate">{label}</span>
              {resource.mimeType && (
                <span className="ml-auto shrink-0 text-xs font-normal text-muted-foreground">
                  {resource.mimeType}
                </span>
              )}
            </summary>
            <pre className="mt-3 max-h-80 overflow-auto whitespace-pre-wrap rounded bg-background/70 p-3 text-xs leading-5 text-muted-foreground">
              {resource.text}
            </pre>
          </details>
        );
      }
      return (
        <div className="flex min-w-0 items-center gap-3 rounded-md border border-border/70 bg-muted/20 px-3 py-2 text-sm">
          <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <div className="truncate font-medium text-foreground">{label}</div>
            <div className="truncate text-xs text-muted-foreground">
              {resource.mimeType ?? t("Binary resource")}
            </div>
          </div>
        </div>
      );
    }
  }
}
