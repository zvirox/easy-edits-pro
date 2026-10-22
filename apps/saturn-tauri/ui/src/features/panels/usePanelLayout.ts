import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { defaultLayout } from "./layouts.ts";
import { PANEL_LAYOUT_EVENT, PANEL_LAYOUT_STORAGE_PREFIX } from "./presets.ts";
import {
  clampPanelRect,
  pixelsToRect,
  sanitizePanelLayout,
} from "./snapping.ts";
import {
  EMPTY_REGION,
  type PanelConstraints,
  type PanelRect,
  type PanelRegion,
} from "./types.ts";

type SetRectReason = "drag" | "resize" | "reset";

interface PanelLayoutOptions {
  workspace: string;
  panelIds: readonly string[];
  /** Panels that start closed until the user opens them from a menu. */
  initialHidden?: readonly string[];
  getConstraints: (panelId: string) => PanelConstraints;
  /** Hidden anchor inside the workspace region; its parent is the floating area. */
  regionRef: React.RefObject<HTMLElement | null>;
}

interface PanelLayoutApi {
  rects: Record<string, PanelRect>;
  collapsed: Record<string, boolean>;
  hidden: Record<string, boolean>;
  pinned: Record<string, boolean>;
  zIndex: Record<string, number>;
  region: PanelRegion;
  setRect: (panelId: string, next: PanelRect, reason: SetRectReason) => void;
  toggleCollapse: (panelId: string) => void;
  togglePin: (panelId: string) => void;
  setClosed: (panelId: string, closed: boolean) => void;
  bringToFront: (panelId: string) => void;
  resetLayout: () => void;
}

const STORAGE_KEY = PANEL_LAYOUT_STORAGE_PREFIX;
const BASE_Z = 500;

function storageKeyFor(workspace: string) {
  return `${STORAGE_KEY}.${workspace}`;
}

function readLocalStorage(workspace: string): Record<string, unknown> | null {
  try {
    const raw = window.localStorage.getItem(storageKeyFor(workspace));
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

function writeLocalStorage(
  workspace: string,
  rects: Record<string, PanelRect>,
  hidden: Record<string, boolean>,
) {
  try {
    window.localStorage.setItem(
      storageKeyFor(workspace),
      JSON.stringify({ rects, hidden }),
    );
  } catch {
    // Storage may be unavailable (quota/private mode); keep running in-memory.
  }
}

/**
 * Owns floating panel layout state for one workspace: rects in viewport
 * fractions, collapse/pin/closed flags, z-order, and persistence through the
 * `save_panel_layout` / `load_panel_layout` commands with a localStorage
 * fallback for non-project surface state.
 */
export function usePanelLayout({
  workspace,
  panelIds,
  initialHidden = [],
  getConstraints,
  regionRef,
}: PanelLayoutOptions): PanelLayoutApi {
  const regionRefState = useRef<PanelRegion>(EMPTY_REGION);
  const [region, setRegion] = useState<PanelRegion>(EMPTY_REGION);
  const [rects, setRects] = useState<Record<string, PanelRect>>(
    () => defaultLayout(workspace) ?? {},
  );
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [hidden, setHidden] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(initialHidden.map((id) => [id, true])),
  );
  const [pinned, setPinned] = useState<Record<string, boolean>>({});
  const [zIndex, setZIndex] = useState<Record<string, number>>({});
  const zCounter = useRef(1);
  const saveTimer = useRef<number | null>(null);

  const panelIdsKey = panelIds.join("|");
  void panelIdsKey;

  // Track the workspace's floating area (the parent of the hidden anchor) so
  // fraction<->pixel conversions stay inside the editor, never over the chrome.
  useEffect(() => {
    const measure = () => {
      const host = regionRef.current?.parentElement;
      if (!host) return;
      const bounds = host.getBoundingClientRect();
      const next: PanelRegion = {
        left: bounds.left,
        top: bounds.top,
        width: bounds.width,
        height: bounds.height,
      };
      regionRefState.current = next;
      setRegion((previous) => {
        const unchanged =
          Math.abs(previous.left - next.left) < 0.5 &&
          Math.abs(previous.top - next.top) < 0.5 &&
          Math.abs(previous.width - next.width) < 0.5 &&
          Math.abs(previous.height - next.height) < 0.5;
        return unchanged ? previous : next;
      });
    };
    measure();
    // Layout settles one frame after mount in some workspaces.
    const settle = window.setTimeout(measure, 60);
    window.addEventListener("resize", measure);
    return () => {
      window.clearTimeout(settle);
      window.removeEventListener("resize", measure);
    };
  }, [regionRef]);
  regionRefState.current = region;

  // Initial load: disk layout first, localStorage as fallback for surface state.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      let loaded: unknown = null;
      try {
        loaded = await invoke<unknown>("load_panel_layout", { workspace });
      } catch {
        loaded = null;
      }
      if (cancelled) return;
      const local = readLocalStorage(workspace);
      if (!loaded && local)
        loaded = { rects: local.rects, hidden: local.hidden };
      setRects((previous) => {
        const sanitised = sanitizePanelLayout(
          loaded,
          panelIds,
          regionRefState.current,
        );
        if (Object.keys(sanitised).length === 0) return previous;
        return { ...previous, ...sanitised };
      });
      if (local && typeof local.hidden === "object" && local.hidden !== null) {
        setHidden({ ...(local.hidden as Record<string, boolean>) });
      }
    })();
    return () => {
      cancelled = true;
    };
    // Load once per workspace mount; panel ids are de-duplicated above.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspace]);

  // Layout presets rewrite localStorage; reload this workspace in place
  // instead of remounting the editor tree.
  useEffect(() => {
    const reload = () => {
      const local = readLocalStorage(workspace);
      if (!local) return;
      const ids = panelIdsKey ? panelIdsKey.split("|") : [];
      const sanitised = sanitizePanelLayout(
        { rects: local.rects, hidden: local.hidden },
        ids,
        regionRefState.current,
      );
      if (Object.keys(sanitised).length > 0) {
        setRects((previous) => ({ ...previous, ...sanitised }));
      }
      if (local.hidden && typeof local.hidden === "object") {
        setHidden({ ...(local.hidden as Record<string, boolean>) });
      }
      setCollapsed({});
    };
    window.addEventListener(PANEL_LAYOUT_EVENT, reload);
    return () => window.removeEventListener(PANEL_LAYOUT_EVENT, reload);
  }, [workspace, panelIdsKey]);

  const refs = useRef({ rects, hidden });
  refs.current = { rects, hidden };

  const schedulePersist = useCallback(() => {
    if (saveTimer.current !== null) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveTimer.current = null;
      const payload = {
        rects: refs.current.rects,
        hidden: refs.current.hidden,
      };
      writeLocalStorage(workspace, payload.rects, payload.hidden);
      void invoke("save_panel_layout", { workspace, layout: payload }).catch(
        () => {
          // Disk persistence is best-effort; localStorage already has it.
        },
      );
    }, 250);
  }, [workspace]);

  const setRect = useCallback(
    (panelId: string, next: PanelRect, _reason: SetRectReason) => {
      void _reason;
      const regionSize = regionRefState.current;
      const constraints = getConstraints(panelId);
      setRects((previous) => {
        // Drag/resize produce region-local pixel coordinates; clamp keeps the
        // panel inside the floating area and honours min/max sizes.
        const pixels = {
          x: next.x * regionSize.width,
          y: next.y * regionSize.height,
          width: next.width * regionSize.width,
          height: next.height * regionSize.height,
        };
        const clamped = pixelsToRect(pixels, regionSize, constraints);
        return { ...previous, [panelId]: clamped };
      });
      schedulePersist();
    },
    [getConstraints, schedulePersist],
  );

  const toggleCollapse = useCallback((panelId: string) => {
    setCollapsed((previous) => ({
      ...previous,
      [panelId]: !previous[panelId],
    }));
  }, []);

  const togglePin = useCallback((panelId: string) => {
    setPinned((previous) => ({ ...previous, [panelId]: !previous[panelId] }));
  }, []);

  const setClosed = useCallback(
    (panelId: string, closed: boolean) => {
      setHidden((previous) => {
        const next = { ...previous, [panelId]: closed };
        if (!closed) delete next[panelId];
        return next;
      });
      schedulePersist();
    },
    [schedulePersist],
  );

  const bringToFront = useCallback((panelId: string) => {
    setZIndex((previous) => {
      if (previous[panelId] === zCounter.current) return previous;
      zCounter.current += 1;
      return { ...previous, [panelId]: BASE_Z + zCounter.current };
    });
  }, []);

  const resetLayout = useCallback(() => {
    const fresh = defaultLayout(workspace) ?? {};
    const clamped: Record<string, PanelRect> = {};
    for (const [id, rect] of Object.entries(fresh)) {
      clamped[id] = clampPanelRect(
        rect,
        regionRefState.current,
        getConstraints(id),
      );
    }
    setRects(clamped);
    setCollapsed({});
    setHidden({});
    setPinned({});
    schedulePersist();
  }, [workspace, getConstraints, schedulePersist]);

  // Keep floating panels inside the floating area when it shrinks.
  useEffect(() => {
    if (region.width === 0 || region.height === 0) return;
    setRects((previous) => {
      const next: Record<string, PanelRect> = {};
      let changed = false;
      for (const [id, rect] of Object.entries(previous)) {
        const clamped = clampPanelRect(rect, region, getConstraints(id));
        if (
          clamped.x !== rect.x ||
          clamped.y !== rect.y ||
          clamped.width !== rect.width ||
          clamped.height !== rect.height
        ) {
          changed = true;
        }
        next[id] = clamped;
      }
      return changed ? next : previous;
    });
    // Runs on every region metric change; constraints are stable per panel id.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [region]);

  return {
    rects,
    collapsed,
    hidden,
    pinned,
    zIndex,
    region,
    setRect,
    toggleCollapse,
    togglePin,
    setClosed,
    bringToFront,
    resetLayout,
  };
}
