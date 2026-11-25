// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';

export interface LinearButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

export const LinearButton: React.FC<LinearButtonProps> = ({
  variant = 'secondary',
  size = 'md',
  className = '',
  children,
  ...props
}) => {
  const sizeStyles = {
    sm: 'h-7 px-2.5 text-xs',
    md: 'h-8 px-3 text-xs',
    lg: 'h-10 px-4 text-sm',
  }[size];

  const variantStyles = {
    primary:
      'bg-[var(--accent-blue)] hover:bg-[var(--accent-blue-hover)] active:bg-[var(--accent-blue-active)] text-[var(--text-on-accent)] font-medium border border-[var(--accent-blue)]',
    secondary:
      'bg-[var(--bg-surface-2)] hover:bg-[var(--bg-bar-idle)] text-[var(--text)] border border-[var(--border)] hover:border-[var(--border-strong)] shadow-[var(--card-shadow)]',
    ghost:
      'bg-transparent hover:bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent',
    danger:
      'bg-[var(--tint-red-bg)] hover:border-[var(--accent-red)] text-[var(--tint-red-fg)] border border-[var(--tint-red-border)]',
  }[variant];

  return (
    <button
      className={`inline-flex items-center justify-center gap-1.5 rounded-[6px] font-sans transition-[background-color,border-color,color,opacity] duration-150 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-blue)] disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${sizeStyles} ${variantStyles} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
