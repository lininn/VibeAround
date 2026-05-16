import { browserBaseUrl } from "@va/client";
import { getAuthToken, isLocalDashboard } from "@/lib/auth";

export function dataUrl(mimeType: string, data: string) {
  return data.startsWith("data:") ? data : `data:${mimeType};base64,${data}`;
}

export function fileNameFromUri(uri: string) {
  const clean = uri.split(/[?#]/)[0]?.replace(/[\\/]+$/, "") ?? uri;
  return clean.split(/[\\/]/).filter(Boolean).pop() ?? uri;
}

export function formatJson(value: unknown) {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function lineCount(text: string | null | undefined) {
  if (!text) return 0;
  return text.split("\n").length;
}

export function proxiedFileUrl(
  uri: string | null | undefined,
  options: {
    name?: string | null;
    mimeType?: string | null;
    inline?: boolean;
  } = {},
) {
  if (!uri || uri.startsWith("data:") || uri.startsWith("blob:")) return uri ?? "";
  if (!isProxyableFileUri(uri)) return uri;

  const params = new URLSearchParams();
  params.set("uri", uri);
  if (options.name) params.set("name", options.name);
  if (options.mimeType) params.set("mime_type", options.mimeType);
  if (options.inline) params.set("inline", "true");
  const token = getAuthToken();
  if (token && !isLocalDashboard()) {
    params.set("token", token);
  }
  return `${browserBaseUrl()}/api/chat/files/download?${params.toString()}`;
}

function isProxyableFileUri(uri: string) {
  return (
    uri.startsWith("file://") ||
    uri.startsWith("http://") ||
    uri.startsWith("https://") ||
    uri.startsWith("/")
  );
}
