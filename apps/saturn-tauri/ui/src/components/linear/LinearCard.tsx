// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { forwardRef } from 'react';

export interface LinearCardProps extends React.HTMLAttributes<HTMLDivElement> {
  level?: 1 | 2 | 3 | 'pane' | 'raised' | 'modal' | 'overlay';
}

export const LinearCard = forwardRef<HTMLDivElement, LinearCardProps>(({
  level = 1,
  className = '',
  children,
  ...props
}, ref) => {
  const normalizedLevel: 1 | 2 | 3 =
    typeof level === 'number'
      ? level
      : level === 'raised'
      ? 2
      : level === 'modal' || level === 'overlay'
      ? 3
      : 1;

  // Flat solid 1px bordered surfaces with shared card elevation token.
  const levelStyles = {
    1: 'bg-surface border-border shadow-card',
    2: 'bg-surface-2 border-border shadow-card',
    3: 'bg-surface border-border-strong shadow-card',
  }[normalizedLevel];

  return (
    <div
      ref={ref}
      className={`rounded-[8px] border transition-[border-color] duration-150 ease-out hover:border-border-strong text-fg ${levelStyles} ${className}`}
      {...props}
    >
      {children}
    </div>
  );
});

LinearCard.displayName = 'LinearCard';
