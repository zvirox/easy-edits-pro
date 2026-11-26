// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';
import { ChevronRight } from 'lucide-react';

export interface BreadcrumbSegment {
  label: string;
  onClick?: () => void;
  active?: boolean;
}

export interface BreadcrumbProps {
  segments: BreadcrumbSegment[];
  right?: React.ReactNode;
  className?: string;
}

export const Breadcrumb: React.FC<BreadcrumbProps> = ({
  segments,
  right,
  className = '',
}) => {
  return (
    <div
      role="navigation"
      aria-label="Breadcrumb"
      className={`linear-breadcrumb h-9 w-full bg-app border-b border-border px-3 flex items-center justify-between gap-3 shrink-0 select-none ${className}`}
    >
      <ol className="flex items-center gap-1.5 min-w-0 text-xs">
        {segments.map((seg, index) => {
          const isLast = index === segments.length - 1 || seg.active;
          return (
            <li key={`${seg.label}-${index}`} className="flex items-center gap-1.5 min-w-0">
              {index > 0 && (
                <ChevronRight
                  className="size-3.5 text-faint shrink-0"
                  aria-hidden="true"
                />
              )}
              {seg.onClick && !isLast ? (
                <button
                  type="button"
                  onClick={seg.onClick}
                  className="px-1.5 py-0.5 rounded-[4px] border border-transparent bg-transparent text-muted hover:text-fg hover:bg-hover truncate transition-[background-color,color] duration-150 ease-out cursor-pointer"
                >
                  {seg.label}
                </button>
              ) : (
                <span
                  aria-current={isLast ? 'page' : undefined}
                  className={`px-1 py-0.5 truncate ${
                    isLast
                      ? 'font-medium text-fg'
                      : 'text-muted'
                  }`}
                >
                  {seg.label}
                </span>
              )}
            </li>
          );
        })}
      </ol>
      {right && <div className="flex items-center gap-2 shrink-0">{right}</div>}
    </div>
  );
};
