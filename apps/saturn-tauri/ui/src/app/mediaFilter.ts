import type { MediaAsset } from "./projectTypes";

/**
 * Filter media items by active bin and search text across name, path, codec, and tags.
 */
export function filterMediaItems(
  media: MediaAsset[] = [],
  search = "",
  activeBinId: number | null = null
): MediaAsset[] {
  const query = search.trim().toLowerCase();
  return media.filter((item) => {
    if (activeBinId !== null && item.bin_id !== activeBinId) return false;
    if (!query) return true;
    const matchName = item.name.toLowerCase().includes(query);
    const matchPath = item.path.toLowerCase().includes(query);
    const matchCodec = item.codec ? item.codec.toLowerCase().includes(query) : false;
    const matchTag = Array.isArray(item.tags)
      ? item.tags.some((t) => typeof t === "string" && t.toLowerCase().includes(query))
      : false;
    return matchName || matchPath || matchCodec || matchTag;
  });
}
