// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import {
  useEffect,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { MasterMeter } from "../features/timeline/MasterMeter";
import { columnRatiosFor } from "./columnRatios";
import {
  PANEL_OPEN_EVENT,
  PANEL_TOGGLE_EVENT,
  type PanelOpenDetail,
} from "../features/dock/panelEvents";
import { DockDrawerContext } from "../features/dock/dockDrawerContext";

export interface WorkspaceShellProps {
  /** Current workspace mode id (`edit`, `color`, `effects`, ...). */
  mode: string;
  /** Left dock card content (re-tabbed per mode; slots are stationary). */
  left: ReactNode;
  /** Right dock card content. */
  right: ReactNode;
  /** The persistent program monitor column (PreviewSection). */
  center: ReactNode;
  /** The persistent vertical tool strip (TimelineToolRail). */
  toolbox: ReactNode;
  /** The persistent timeline (TimelineSection). */
  timeline: ReactNode;
}

/** Right dock panel IDs across all suites that route into the right dock card/drawer. */
const RIGHT_DOCK_PANELS = new Set([
  // Edit suite
  "inspector",
  "sheet",
  "agent",
  "review",
  "script",
  "telemetry",
  "markers",
  "captions",
  "graphics",
  "nodegraph",
  // Color suite
  "grade",
  "looks",
  // Effects suite
  "effects-stack",
  // Fairlight suite
  "fairlight-dsp",
  "fairlight-automation",
  // Fusion suite
  "fusion-canvas",
  "fusion-node-inspector",
  // Curves / Keyframes suite
  "keyframes-curves",
  "keyframes-tangents",
]);

/**
 * Persistent 2-row NLE shell (plan v4 G3, docs/eep-glass-cards-mock.html).
 *
 * Row 1: left dock card | program monitor | right dock card.
 * Row 2: one full-width NLE strip — toolbox | timeline | master meter.
 *
 * Only the dock-tab content changes per workspace mode; the program monitor,
 * toolbox, timeline, and master meter are mounted once by the router and
 * never unmount when the mode switches (directive 5, geometry invariance).
 */
export function WorkspaceShell({
  mode,
  left,
  right,
  center,
  toolbox,
  timeline,
}: WorkspaceShellProps) {
  // Plan §5: the mode picks its column ratio from the small fixed table;
  // only the dock columns change, the persistent zones never remount.
  const [leftRatio, , rightRatio] = columnRatiosFor(mode);
  const ratioVars = {
    "--shell-left": `${leftRatio}%`,
    "--shell-right": `${rightRatio}%`,
  } as CSSProperties;

  // UI-10: Responsive dock degradation state on <=1280px viewports.
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);

  // Reset drawer state on workspace mode switch to maintain clean resting geometry
  useEffect(() => {
    setIsDrawerOpen(false);
  }, [mode]);

  // Listen for PANEL_OPEN_EVENT and PANEL_TOGGLE_EVENT
  useEffect(() => {
    const handleOpen = (e: Event) => {
      const detail = (e as CustomEvent<PanelOpenDetail>).detail;
      if (detail?.id && RIGHT_DOCK_PANELS.has(detail.id)) {
        if (typeof window !== "undefined" && window.innerWidth <= 1280) {
          setIsDrawerOpen(true);
        }
      }
    };

    const handleToggle = (e: Event) => {
      const detail = (e as CustomEvent<PanelOpenDetail>).detail;
      if (detail?.id && RIGHT_DOCK_PANELS.has(detail.id)) {
        if (typeof window !== "undefined" && window.innerWidth <= 1280) {
          setIsDrawerOpen((prev) => !prev);
        }
      }
    };

    window.addEventListener(PANEL_OPEN_EVENT, handleOpen);
    window.addEventListener(PANEL_TOGGLE_EVENT, handleToggle);
    return () => {
      window.removeEventListener(PANEL_OPEN_EVENT, handleOpen);
      window.removeEventListener(PANEL_TOGGLE_EVENT, handleToggle);
    };
  }, []);

  // Escape key dismiss when drawer is open
  useEffect(() => {
    if (!isDrawerOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsDrawerOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isDrawerOpen]);

  // Auto-close drawer when resizing above 1280px
  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth > 1280 && isDrawerOpen) {
        setIsDrawerOpen(false);
      }
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [isDrawerOpen]);

  return (
    <DockDrawerContext.Provider
      value={{
        isDrawerOpen,
        closeDrawer: () => setIsDrawerOpen(false),
        openDrawer: () => setIsDrawerOpen(true),
        toggleDrawer: () => setIsDrawerOpen((v) => !v),
      }}
    >
      <main
        className={`workspace-shell${isDrawerOpen ? " has-drawer-open" : ""}`}
        data-workspace={mode}
        aria-label={`${mode} workspace`}
        style={ratioVars}
      >
        <div className="shell-top">
          {left}
          <section className="shell-center" aria-label="Program monitor">
            {center}
            <button
              type="button"
              className="dock-drawer-toggle"
              aria-label="Toggle inspector dock drawer"
              aria-expanded={isDrawerOpen}
              onClick={() => setIsDrawerOpen((open) => !open)}
              title="Toggle inspector dock drawer"
            >
              <span className="dock-drawer-toggle-icon" aria-hidden="true">
                ◨
              </span>
              <span className="dock-drawer-toggle-label">Inspector</span>
            </button>
          </section>
          {right}
        </div>
        <div className="shell-nle-row">
          <aside className="shell-toolbox" aria-label="Editing toolbox">
            {toolbox}
          </aside>
          <div className="shell-timeline">{timeline}</div>
          <MasterMeter />
        </div>
      </main>
    </DockDrawerContext.Provider>
  );
}

export default WorkspaceShell;
