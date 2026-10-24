import type { CSSProperties, ReactNode } from "react";
import { MasterMeter } from "../features/timeline/MasterMeter";
import { columnRatiosFor } from "./columnRatios";

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
export function WorkspaceShell({ mode, left, right, center, toolbox, timeline }: WorkspaceShellProps) {
  // Plan §5: the mode picks its column ratio from the small fixed table;
  // only the dock columns change, the persistent zones never remount.
  const [leftRatio, , rightRatio] = columnRatiosFor(mode);
  const ratioVars = {
    "--shell-left": `${leftRatio}%`,
    "--shell-right": `${rightRatio}%`,
  } as CSSProperties;
  return (
    <main className="workspace-shell" data-workspace={mode} style={ratioVars}>
      <div className="shell-top">
        {left}
        <section className="shell-center" aria-label="Program monitor">
          {center}
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
  );
}

export default WorkspaceShell;
