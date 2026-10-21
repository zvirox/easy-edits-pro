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
  style,
}: StudioPanelProps) {
  return (
    <section className={`studio-panel glass ${className}`} data-elev="1" style={style}>
      {(title != null || actions != null) && (
        <header className={`studio-panel-header ${headerClassName}`}>
          <div className="studio-panel-title">
            {typeof title === "string" ? <span>{title}</span> : title}
          </div>
          {actions && <div className="studio-panel-actions">{actions}</div>}
        </header>
      )}
      <div
        className={`studio-panel-body ${scrollable ? "is-scrollable scrollbar-thin" : "no-scroll"} ${
          noPadding ? "no-padding" : ""
        } ${bodyClassName}`}
      >
        {children}
      </div>
    </section>
  );
}
