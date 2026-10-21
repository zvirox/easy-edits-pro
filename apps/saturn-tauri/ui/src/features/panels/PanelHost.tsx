import { useEffect, useMemo, useRef } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import { FloatingPanel } from "./FloatingPanel.tsx";
import { usePanelLayout } from "./usePanelLayout.ts";
import { PANEL_OPEN_EVENT, PANEL_RESET_EVENT, PANEL_TOGGLE_EVENT } from "./presets.ts";
import {
  DEFAULT_CONSTRAINTS,
  type PanelConstraints,
  type PanelRect,
  type PanelRegion,
} from "./types.ts";

export interface PanelHostProps {
  workspace: string;
  entries: PanelEntry[];
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
export function PanelHost({ workspace, entries }: PanelHostProps) {
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
    getConstraints,
    regionRef,
  });
  const region: PanelRegion = layout.region;

  // Gestures (Trim Monitor) and View ▸ Panels talk to the host by event so the
  // menu and timeline hooks stay independent of the layout store.
  const setClosed = layout.setClosed;
  const hidden = layout.hidden;
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
      if (id && panelIds.includes(id)) setClosed(id, !hidden[id]);
    };
    const reset = () => layout.resetLayout();
    window.addEventListener(PANEL_OPEN_EVENT, open);
    window.addEventListener(PANEL_TOGGLE_EVENT, toggle);
    window.addEventListener(PANEL_RESET_EVENT, reset);
    return () => {
      window.removeEventListener(PANEL_OPEN_EVENT, open);
      window.removeEventListener(PANEL_TOGGLE_EVENT, toggle);
      window.removeEventListener(PANEL_RESET_EVENT, reset);
    };
  }, [panelIds, setClosed, hidden]);

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

  return (
    <>
      <div ref={regionRef} hidden aria-hidden="true" />
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
            zIndex={layout.zIndex[entry.id] ?? PANEL_BASE_Z + index}
            collapsed={collapsed}
            pinned={Boolean(layout.pinned[entry.id])}
            otherRects={activeRects.filter((item) => item.id !== entry.id)}
            onMove={(next) => layout.setRect(entry.id, next, "drag")}
            onRaise={() => layout.bringToFront(entry.id)}
            onToggleCollapse={() => layout.toggleCollapse(entry.id)}
            onTogglePin={() => layout.togglePin(entry.id)}
            onClose={() => layout.setClosed(entry.id, true)}
            onOpen={() => layout.toggleCollapse(entry.id)}
          >
            {entry.render({ close: () => layout.setClosed(entry.id, true) })}
          </FloatingPanel>,
          document.body,
        );
      })}
      {entries.some((entry) => layout.hidden[entry.id]) && (
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
