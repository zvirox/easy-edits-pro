// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';

export interface DonutLegendItem {
  label: string;
  value: string | number;
  color: string;
  subValue?: string;
}

export interface DonutLegendProps {
  items: DonutLegendItem[];
  className?: string;
}

export const DonutLegend: React.FC<DonutLegendProps> = ({ items, className = '' }) => {
  return (
    <div className={`flex flex-col w-full ${className}`}>
      {items.map((item, index) => (
        <div
          key={index}
          className={`flex items-center justify-between text-xs gap-3 py-2 ${
            index < items.length - 1 ? 'border-b border-dashed border-[var(--border)]' : ''
          }`}
        >
          <div className="flex items-center gap-2 min-w-0">
            <span
              className="size-2.5 rounded-[2px] shrink-0"
              style={{ backgroundColor: item.color }}
              aria-hidden="true"
            />
            <span className="text-[var(--text-muted)] truncate">{item.label}</span>
          </div>
          <div className="flex items-center gap-2 shrink-0 font-mono">
            <span className="font-semibold text-[var(--text)] tabular-nums">{item.value}</span>
            {item.subValue && (
              <span className="text-[var(--text-faint)] text-[11px] tabular-nums">
                ({item.subValue})
              </span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
};
