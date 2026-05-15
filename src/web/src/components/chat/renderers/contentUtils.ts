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
