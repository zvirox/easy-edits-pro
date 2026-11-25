// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react';

export interface SidebarNavItem {
  id: string;
  label: string;
  icon: React.ReactNode;
  badge?: string;
  accentColor?: string;
  shortcut?: string;
}

export interface SidebarSection {
  title: string;
  items: SidebarNavItem[];
}

export interface SidebarProps {
  sections: SidebarSection[];
  activeId: string;
  onSelect: (id: string) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  header?: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
}

export const Sidebar: React.FC<SidebarProps> = ({
  sections,
  activeId,
  onSelect,
  collapsed,
  onToggleCollapse,
  header,
  footer,
  className = '',
}) => {
  return (
    <aside
      aria-label="Workspace navigation"
      data-collapsed={collapsed ? 'true' : 'false'}
      className={`linear-sidebar shrink-0 h-full bg-[var(--bg-app)] border-r border-[var(--border)] flex flex-col select-none ${
        collapsed ? 'w-14' : 'w-[240px]'
      } ${className}`}
    >
      {/* Top header / project switcher */}
      <div className="h-10 px-2.5 flex items-center justify-between border-b border-[var(--border)] shrink-0 gap-2">
        {!collapsed && (
          <div className="min-w-0 flex-1 flex items-center gap-2">{header}</div>
        )}
        <button
          type="button"
          onClick={onToggleCollapse}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          aria-expanded={!collapsed}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          className="size-7 p-0 inline-flex items-center justify-center rounded-[6px] border border-transparent bg-transparent text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-[background-color,color] duration-150 ease-out cursor-pointer shrink-0"
        >
          {collapsed ? (
            <PanelLeftOpen className="size-3.5 shrink-0" aria-hidden="true" />
          ) : (
            <PanelLeftClose className="size-3.5 shrink-0" aria-hidden="true" />
          )}
        </button>
      </div>

      {/* Navigation sections */}
      <nav className="flex-1 overflow-y-auto overflow-x-hidden py-2 px-2 flex flex-col gap-3 scrollbar-thin">
        {sections.map((section) => (
          <div key={section.title} className="flex flex-col gap-0.5">
            {!collapsed && (
              <div className="px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-[var(--text-faint)]">
                {section.title}
              </div>
            )}
            {section.items.map((item) => {
              const isActive = item.id === activeId;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onSelect(item.id)}
                  aria-current={isActive ? 'page' : undefined}
                  title={
                    collapsed
                      ? `${item.label}${item.shortcut ? ` (${item.shortcut})` : ''}`
                      : item.shortcut
                  }
                  className={`h-9 w-full flex items-center gap-2.5 px-2.5 rounded-[6px] text-[13px] font-medium transition-[background-color,color,border-color] duration-150 ease-out cursor-pointer border ${
                    isActive
                      ? 'bg-[var(--bg-surface-2)] border-[var(--border)] text-[var(--text)]'
                      : 'bg-transparent border-transparent text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface)]'
                  } ${collapsed ? 'justify-center px-0' : ''}`}
                >
                  <span
                    className={`size-4 shrink-0 flex items-center justify-center ${
                      isActive ? 'text-[var(--accent-blue)]' : ''
                    }`}
                    aria-hidden="true"
                  >
                    {item.icon}
                  </span>
                  {!collapsed && (
                    <>
                      <span className="truncate flex-1 text-left">{item.label}</span>
                      {item.badge && (
                        <span className="px-1 py-0.5 text-[10px] font-semibold rounded-[4px] bg-[var(--tint-purple-bg)] text-[var(--tint-purple-fg)] border border-[var(--tint-purple-border)] leading-none">
                          {item.badge}
                        </span>
                      )}
                      {!item.badge && item.shortcut && (
                        <span className="text-[11px] font-mono text-[var(--text-faint)]">
                          {item.shortcut}
                        </span>
                      )}
                    </>
                  )}
                </button>
              );
            })}
          </div>
        ))}
      </nav>

      {footer && (
        <div className="p-2 border-t border-[var(--border)] shrink-0">
          {footer}
        </div>
      )}
    </aside>
  );
};
