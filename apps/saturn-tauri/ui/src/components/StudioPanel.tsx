// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import type { CSSProperties, ReactNode } from "react";

export interface StudioPanelProps {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
  headerClassName?: string;
  scrollable?: boolean;
  noPadding?: boolean;
  collapsed?: boolean;
  style?: CSSProperties;
}

export function StudioPanel({
  title,
  actions,
  children,
  className = "",
  bodyClassName = "",
  headerClassName = "",
  scrollable = true,
  noPadding = false,
  collapsed = false,
  style,
}: StudioPanelProps) {
  return (
    <section
      className={`studio-panel glass ${collapsed ? "is-collapsed " : ""}${className}`}
      data-elev="1"
      style={style}
    >
      {(title != null || actions != null) && (
        <header className={`studio-panel-header ${headerClassName}`}>
          <div className="studio-panel-title">
            {typeof title === "string" ? <span>{title}</span> : title}
          </div>
          {actions && <div className="studio-panel-actions">{actions}</div>}
        </header>
      )}
      {!collapsed && (
        <div
          className={`studio-panel-body ${scrollable ? "is-scrollable scrollbar-thin" : "no-scroll"} ${
            noPadding ? "no-padding" : ""
          } ${bodyClassName}`}
        >
          {children}
        </div>
      )}
    </section>
  );
}
