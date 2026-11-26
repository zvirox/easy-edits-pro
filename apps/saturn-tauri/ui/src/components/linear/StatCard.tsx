// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';
import { ArrowUpRight, ArrowDownRight } from 'lucide-react';
import { LinearCard } from './LinearCard';

export interface StatCardProps {
  title: string;
  value: string | number;
  unit?: string;
  delta?: {
    value: string | number;
    positive?: boolean;
    comparison?: string;
  };
  icon?: React.ReactNode;
  iconBgClass?: string;
  iconColorClass?: string;
  compact?: boolean;
  className?: string;
}

export const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  unit,
  delta,
  icon,
  iconBgClass = 'bg-primary-tint border border-primary',
  iconColorClass = 'text-primary-text',
  compact = false,
  className = '',
}) => {
  return (
    <LinearCard
      level={1}
      className={`${compact ? 'p-3.5' : 'p-5'} min-w-0 flex flex-col justify-between ${className}`}
    >
      {/* Top row: Title and solid precomputed Icon Chip */}
      <div className="flex items-center justify-between gap-1.5 min-w-0">
        <span className="text-[11px] font-medium uppercase tracking-wider text-fg-2 select-none leading-tight min-w-0">
          {title}
        </span>
        {icon && (
          <div
            className={`${compact ? 'size-6' : 'size-7'} rounded-[6px] flex items-center justify-center shrink-0 ${iconBgClass} ${iconColorClass}`}
          >
            {icon}
          </div>
        )}
      </div>

      {/* Middle row: tabular figure + muted unit */}
      <div
        className={`${
          compact
            ? 'mt-2 flex flex-wrap items-baseline gap-x-1.5 gap-y-0.5 min-w-0'
            : 'mt-3 flex flex-wrap items-baseline gap-1.5 min-w-0'
        }`}
      >
        <span
          className={`${
            compact ? 'text-[18px] leading-tight' : 'text-[28px] leading-none'
          } font-semibold tracking-tight text-fg tabular-nums font-sans`}
        >
          {value}
        </span>
        {unit && (
          <span className="text-[11px] font-medium text-muted uppercase tracking-wide whitespace-nowrap">
            {unit}
          </span>
        )}
      </div>

      {/* Bottom row: solid precomputed Delta chip + comparison */}
      {delta && (
        <div className={`${compact ? 'mt-2.5' : 'mt-4'} flex items-center gap-1.5 min-w-0`}>
          <span
            className={`inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded-[4px] border text-[11px] font-medium tabular-nums ${
              delta.positive
                ? 'bg-chip-sky text-sky-text border-sky'
                : 'bg-chip-red text-red-text border-red'
            }`}
          >
            {delta.positive ? (
              <ArrowUpRight className="size-3 shrink-0" aria-hidden="true" />
            ) : (
              <ArrowDownRight className="size-3 shrink-0" aria-hidden="true" />
            )}
            {delta.value}
          </span>
          {delta.comparison && (
            <span className="text-xs text-muted truncate">
              {delta.comparison}
            </span>
          )}
        </div>
      )}
    </LinearCard>
  );
};
