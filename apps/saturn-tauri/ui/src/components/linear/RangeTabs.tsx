// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';

export interface RangeTabsProps {
  options?: string[];
  active: string;
  onChange: (tab: string) => void;
  className?: string;
}

const DEFAULT_OPTIONS = ['1W', '1M', '3W', 'YTD', 'Total'];

export const RangeTabs: React.FC<RangeTabsProps> = ({
  options = DEFAULT_OPTIONS,
  active,
  onChange,
  className = '',
}) => {
  return (
    <div
      role="tablist"
      className={`inline-flex items-center gap-0.5 p-0.5 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] select-none ${className}`}
    >
      {options.map((tab) => {
        const isActive = tab === active;
        return (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={isActive}
            onClick={() => onChange(tab)}
            className={`px-2.5 py-1 text-xs rounded-[4px] font-medium transition-[background-color,color,border-color] duration-150 ease-out cursor-pointer ${
              isActive
                ? 'bg-[var(--bg-surface)] text-[var(--text)] border border-[var(--border)] shadow-[var(--card-shadow)]'
                : 'text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-bar-idle)] border border-transparent'
            }`}
          >
            {tab}
          </button>
        );
      })}
    </div>
  );
};
