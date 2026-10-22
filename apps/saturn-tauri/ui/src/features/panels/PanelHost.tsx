import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import { FloatingPanel } from "./FloatingPanel.tsx";
import { usePanelLayout } from "./usePanelLayout.ts";
import {
  PANEL_OPEN_EVENT,
  PANEL_REOPEN_EVENT,
  PANEL_RESET_EVENT,
  PANEL_TOGGLE_EVENT,
} from "./presets.ts";
import { zonePixels, zoneRect, type DockZone } from "./docking.ts";
import { effectiveZIndex } from "./zOrder.ts";
import {
  DEFAULT_CONSTRAINTS,
  type PanelConstraints,
  type PanelRect,
  type PanelRegion,
} from "./types.ts";

export interface PanelHostProps {
  workspace: string;
  entries: PanelEntry[];
  /** Panels that stay closed until the user opens them from a menu. */
  defaultHidden?: readonly string[];
  /**
   * Called when a panel becomes hidden through an engine close action (shell
   * ✕, entry close, or a close toggle) so a parent can keep its own open state
   * in sync (the AI chat drawer owns its open flag at App level).
   */
  onPanelClosed?: (panelId: string) => void;
  /**
   * Hosts whose panels are opened from menus (the AI Assistant) rather than
   * from a dock rail can suppress the rail entirely.
   */
  showRail?: boolean;
}

export interface PanelEntry {
  id: string;
  title: string;
  constraints?: PanelConstraints;
  render: (api: { close: () => void }) => ReactNode;
}

const DEFAULT_RECT: PanelRect = { x: 0.02, y: 0.02, width: 0.3, height: 0.5 };

const PANEL_BASE_Z = 500;

/**
 * Owns the floating-panel layout for one workspace and portals every panel to
 * document.body so z-index never fights the editor grid. The dock rail lets a
 * closed panel be reopened; "Reset layout" restores workspace defaults.
 */
export function PanelHost({
  workspace,
  entries,
  defaultHidden,
  onPanelClosed,
  showRail = true,
}: PanelHostProps) {
  // Hidden anchor inside the workspace markup; the layout hook measures the
  // anchor's parent (the editor region) so panels never cover the app chrome.
  const regionRef = useRef<HTMLDivElement>(null);
  const panelIds = useMemo(() => entries.map((entry) => entry.id), [entries]);
  const constraints = useMemo(() => {
    const map: Record<string, PanelConstraints> = {};
    for (const entry of entries) {
      map[entry.id] = entry.constraints ?? DEFAULT_CONSTRAINTS;
    }
    return map;
  }, [entries]);
  const getConstraints = (panelId: string): PanelConstraints =>
    constraints[panelId] ?? DEFAULT_CONSTRAINTS;

  const layout = usePanelLayout({
    workspace,
    panelIds,
    initialHidden: defaultHidden,
    getConstraints,
    regionRef,
  });
  const region: PanelRegion = layout.region;

  // Gestures (Trim Monitor) and View ▸ Panels talk to the host by event so the
  // menu and timeline hooks stay independent of the layout store.
  const setClosed = layout.setClosed;
  const hidden = layout.hidden;
  /** Engine-side close; also notifies the parent so its state stays in sync. */
  const closePanel = useCallback(
    (panelId: string) => {
      setClosed(panelId, true);
      onPanelClosed?.(panelId);
    },
    [setClosed, onPanelClosed],
  );
  const reopen = useCallback(() => {
    for (const id of panelIds) setClosed(id, false);
  }, [panelIds, setClosed]);
  useEffect(() => {
    const panelIdOf = (event: Event): string | null => {
      const detail = (event as CustomEvent<{ id?: string }>).detail;
      return detail && typeof detail.id === "string" ? detail.id : null;
    };
    const open = (event: Event) => {
      const id = panelIdOf(event);
      if (id && panelIds.includes(id)) setClosed(id, false);
    };
    const toggle = (event: Event) => {
      const id = panelIdOf(event);
      if (id && panelIds.includes(id)) {
        if (hidden[id]) setClosed(id, false);
        else closePanel(id);
      }
    };
    const reset = () => layout.resetLayout();
    window.addEventListener(PANEL_OPEN_EVENT, open);
    window.addEventListener(PANEL_REOPEN_EVENT, reopen);
    window.addEventListener(PANEL_TOGGLE_EVENT, toggle);
    window.addEventListener(PANEL_RESET_EVENT, reset);
    return () => {
      window.removeEventListener(PANEL_OPEN_EVENT, open);
      window.removeEventListener(PANEL_REOPEN_EVENT, reopen);
      window.removeEventListener(PANEL_TOGGLE_EVENT, toggle);
      window.removeEventListener(PANEL_RESET_EVENT, reset);
    };
  }, [panelIds, setClosed, hidden, reopen, closePanel, layout.resetLayout]);

  // Dock zones (G4a deferred): track the zone under a drag so a ghost preview
  // can highlight it, then reflow the panel on drop.
  const [dockTarget, setDockTarget] = useState<DockZone | null>(null);
  const handleDockTarget = useCallback((zone: DockZone | null) => {
    setDockTarget(zone);
  }, []);
  const handleDropDock = useCallback(
    (panelId: string, zone: DockZone | null) => {
      setDockTarget(null);
      if (!zone) return;
      layout.setRect(panelId, zoneRect(zone, layout.region), "drag");
    },
    [layout],
  );

  const activeRects = entries
    .filter((entry) => !layout.hidden[entry.id])
    .map((entry) => {
      const rect = layout.rects[entry.id] ?? DEFAULT_RECT;
      return {
        id: entry.id,
        rect: {
          x: rect.x * region.width,
          y: rect.y * region.height,
          width: rect.width * region.width,
          height: rect.height * region.height,
        },
      };
    });

  // Pin semantics: derive z-order so pinned panels always render above.
  const zIndexMap = useMemo(
    () => effectiveZIndex(layout.zIndex, layout.pinned),
    [layout.zIndex, layout.pinned],
  );

  return (
    <>
      <div ref={regionRef} hidden aria-hidden="true" />
      {dockTarget !== null && region.width > 0 &&
        createPortal(
          <div
            className="panel-dock-ghost"
            style={{
              left: `${(region.left + zonePixels(dockTarget, region).x).toFixed(2)}px`,
              top: `${(region.top + zonePixels(dockTarget, region).y).toFixed(2)}px`,
              width: `${zonePixels(dockTarget, region).width.toFixed(2)}px`,
              height: `${zonePixels(dockTarget, region).height.toFixed(2)}px`,
            }}
          />,
          document.body,
        )}
      {entries.map((entry, index) => {
        if (layout.hidden[entry.id]) return null;
        const rect = layout.rects[entry.id] ?? DEFAULT_RECT;
        const collapsed = Boolean(layout.collapsed[entry.id]);
        return createPortal(
          <FloatingPanel
            key={entry.id}
            panelId={entry.id}
            title={entry.title}
            rect={rect}
            region={region}
            constraints={constraints[entry.id]}
            zIndex={zIndexMap[entry.id] ?? PANEL_BASE_Z + index}
            collapsed={collapsed}
            pinned={Boolean(layout.pinned[entry.id])}
            otherRects={activeRects.filter((item) => item.id !== entry.id)}
            onMove={(next) => layout.setRect(entry.id, next, "drag")}
            onRaise={() => layout.bringToFront(entry.id)}
            onToggleCollapse={() => layout.toggleCollapse(entry.id)}
            onTogglePin={() => layout.togglePin(entry.id)}
            onClose={() => closePanel(entry.id)}
            onOpen={() => layout.toggleCollapse(entry.id)}
            onDockTarget={handleDockTarget}
            onDropDock={(zone) => handleDropDock(entry.id, zone)}
          >
            {entry.render({ close: () => closePanel(entry.id) })}
          </FloatingPanel>,
          document.body,
        );
      })}
      {showRail && entries.some((entry) => layout.hidden[entry.id]) && (
        <div className="panel-rail" data-workspace={workspace}>
          <span className="panel-rail-label">Panels</span>
          {entries
            .filter((entry) => layout.hidden[entry.id])
            .map((entry) => (
              <button
                key={entry.id}
                type="button"
                className="panel-rail-button"
                onClick={() => layout.setClosed(entry.id, false)}
              >
                {entry.title}
              </button>
            ))}
          <button
            type="button"
            className="panel-rail-button panel-rail-reset"
            onClick={layout.resetLayout}
          >
            Reset layout
          </button>
        </div>
      )}
    </>
  );
}
