/**
 * Pin semantics (Phase 4 deferred, G4a "Pin — exclude from auto-dock" and the
 * existing "stays above workspace changes" button contract).
 *
 * Pinned panels must always render above unpinned ones, whatever order
 * panels were raised in. Rather than racing click handlers against pin state,
 * the effective z-order is derived at render time from the raise history plus
 * the pin flags.
 */

/** Base z for panels; keep in sync with usePanelLayout's BASE_Z. */
export const PANEL_Z_BASE = 500;

/**
 * Rebuild the z-index map so every pinned panel sits above every unpinned
 * panel. The relative order inside each group (raise history) is preserved.
 * Returns the input map untouched when nothing is pinned.
 */
export function effectiveZIndex(
  zIndex: Record<string, number>,
  pinned: Record<string, boolean>,
): Record<string, number> {
  const pinnedIds = Object.keys(pinned).filter((id) => pinned[id]);
  if (pinnedIds.length === 0) return zIndex;

  const unpinnedZ = Object.keys(zIndex)
    .filter((id) => !pinned[id])
    .map((id) => zIndex[id] ?? PANEL_Z_BASE);
  const pinnedZ = pinnedIds.map((id) => zIndex[id] ?? PANEL_Z_BASE);

  // Lowest possible z a pinned panel may take: above every unpinned panel and
  // never below where it already sits (so pinning can't drop a panel down).
  const floor = Math.max(PANEL_Z_BASE, ...unpinnedZ, ...pinnedZ) + 1;

  const next = { ...zIndex };
  // Keep the pinned panels' relative raise order by sorting on current z.
  for (const [index, id] of pinnedIds
    .slice()
    .sort((a, b) => (zIndex[a] ?? PANEL_Z_BASE) - (zIndex[b] ?? PANEL_Z_BASE))
    .entries()) {
    next[id] = floor + index;
  }
  return next;
}
