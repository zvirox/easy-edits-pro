/**
 * Phase 4 floating panel engine types.
 *
 * Layout is stored in viewport fractions (0-1) so a layout made on one display
 * stays valid on another. Pixel maths only happens at render time.
 */

export interface PanelRect {
  /** Distance from the left viewport edge as a fraction of viewport width. */
  x: number;
  /** Distance from the top viewport edge as a fraction of viewport height. */
  y: number;
  /** Panel width as a fraction of viewport width. */
  width: number;
  /** Panel height as a fraction of viewport height. */
  height: number;
}

/** Per-panel clamps, in viewport fractions. */
export interface PanelConstraints {
  minWidth: number;
  minHeight: number;
  maxWidth: number;
  maxHeight: number;
}

export type PanelRectMap = Record<string, PanelRect>;

export interface PanelLayoutState {
  rects: PanelRectMap;
  collapsed: Record<string, boolean>;
  closed: Record<string, boolean>;
  pinned: Record<string, boolean>;
}

export const DEFAULT_CONSTRAINTS: PanelConstraints = {
  minWidth: 0.14,
  minHeight: 0.18,
  maxWidth: 0.5,
  maxHeight: 0.92,
};

/**
 * The floating area panels live in: the editor region below the app chrome,
 * measured in window pixels. Rect fractions are relative to this region so a
 * panel can never drift over the menu bar or status bar.
 */
export interface PanelRegion {
  left: number;
  top: number;
  width: number;
  height: number;
}

export const EMPTY_REGION: PanelRegion = {
  left: 0,
  top: 0,
  width: 0,
  height: 0,
};

/** Snap threshold in pixels (Phase 4 plan: ~12px). */
export const SNAP_THRESHOLD_PX = 12;

/** Interior margin keeps panels inside the visible track area. */
export const VIEWPORT_MARGIN_PX = 8;
