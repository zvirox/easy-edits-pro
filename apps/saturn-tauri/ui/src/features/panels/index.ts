/**
 * Phase 4 floating panel engine — public API.
 *
 * Panels are pushed through `PanelHost` with entries; `FloatingPanel` is the
 * glass shell. Persistence lives in `usePanelLayout` (Rust commands with a
 * localStorage fallback), geometry in `snapping`, defaults in `layouts`.
 */
export {
  PanelHost,
  type PanelEntry,
  type PanelHostProps,
} from "./PanelHost.tsx";
export { FloatingPanel, type FloatingPanelProps } from "./FloatingPanel.tsx";
export {
  clampPanelRect,
  snapEdge,
  snapRect,
  sanitizePanelLayout,
} from "./snapping.ts";
export { defaultLayout, defaultPanels } from "./layouts.ts";
export {
  SNAP_THRESHOLD_PX,
  DEFAULT_CONSTRAINTS,
  type PanelRect,
  type PanelConstraints,
} from "./types.ts";
