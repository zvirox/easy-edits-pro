// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';

export interface ProgressRowProps {
  label: string;
  value: number; // 0 - 100
  displayValue?: string;
  color?: string;
  className?: string;
}

export const ProgressRow: React.FC<ProgressRowProps> = ({
  label,
  value,
  displayValue,
  color = 'var(--primary)',
  className = '',
}) => {
  const clamped = Math.min(100, Math.max(0, value));

  return (
    <div className={`flex flex-col gap-1.5 w-full ${className}`}>
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-fg truncate">{label}</span>
        <span className="font-mono text-muted tabular-nums shrink-0">
          {displayValue ?? `${clamped}%`}
        </span>
      </div>
      <div className="h-1 w-full rounded-[4px] bg-track overflow-hidden">
        <div
          className="h-full w-full rounded-[4px] origin-left transition-transform duration-150 ease-out"
          style={{ transform: `scaleX(${clamped / 100})`, backgroundColor: color }}
        />
      </div>
    </div>
  );
};
