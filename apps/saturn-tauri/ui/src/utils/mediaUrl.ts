export function buildMediaStreamUrl(
  filePath: string,
  mediaServerPort: number,
  mediaServerToken?: string
): string {
  if (!filePath || mediaServerPort <= 0) return "";
  const tokenPart = mediaServerToken ? `&token=${encodeURIComponent(mediaServerToken)}` : "";
  return `http://127.0.0.1:${mediaServerPort}/media?path=${encodeURIComponent(filePath)}${tokenPart}`;
}
