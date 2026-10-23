import { useEffect, useState, type ReactNode } from "react";
import {
  PANEL_OPEN_EVENT,
  PANEL_TOGGLE_EVENT,
  type PanelOpenDetail,
} from "./panelEvents";

export interface DockTab {
  /** Stable panel id — also the id used by PANEL_OPEN/TOGLE events. */
  id: string;
  title: string;
  render: () => ReactNode;
}

export interface DockCardProps {
  className?: string;
  tabs: DockTab[];
  /** Accessible name for the card and its tablist. */
  ariaLabel?: string;
}

/**
 * Stationary glass dock card (plan v4 §B): a `.glass.card` with a 38px tab
 * header and exactly one visible body. Panels can no longer float, close, or
 * be pinned; PANEL_OPEN/TOGGLE events focus the owning tab instead. Only the active
 * tab stays mounted, so hidden panels (scopes, canvases) stop their work.
 */
export function DockCard({ className, tabs, ariaLabel }: DockCardProps) {
  const [activeId, setActiveId] = useState(() => tabs[0]?.id ?? "");
  const active = tabs.find((tab) => tab.id === activeId) ?? tabs[0];

  useEffect(() => {
    const focus = (event: Event) => {
      const detail = (event as CustomEvent<PanelOpenDetail>).detail;
      if (detail?.id && tabs.some((tab) => tab.id === detail.id)) {
        setActiveId(detail.id);
      }
    };
    window.addEventListener(PANEL_OPEN_EVENT, focus);
    window.addEventListener(PANEL_TOGGLE_EVENT, focus);
    return () => {
      window.removeEventListener(PANEL_OPEN_EVENT, focus);
      window.removeEventListener(PANEL_TOGGLE_EVENT, focus);
    };
  }, [tabs]);

  return (
    <section
      className={`glass card dock-card${className ? ` ${className}` : ""}`}
      aria-label={ariaLabel}
    >
      <header
        className="card-h dock-card-tabs"
        role="tablist"
        aria-label={ariaLabel ? `${ariaLabel} tabs` : "Panel tabs"}
      >
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={tab.id === active?.id}
            className="tab"
            onClick={() => setActiveId(tab.id)}
          >
            {tab.title}
          </button>
        ))}
      </header>
      {active ? (
        <div
          className="card-b dock-card-body"
          role="tabpanel"
          aria-label={active.title}
        >
          {active.render()}
        </div>
      ) : null}
    </section>
  );
}

export default DockCard;
