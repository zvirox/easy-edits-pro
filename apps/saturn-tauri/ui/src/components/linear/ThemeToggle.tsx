// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from 'react';
import { Sun, Moon } from 'lucide-react';

export interface LinearThemeToggleProps {
  theme: 'dark' | 'light';
  onToggle: () => void;
  className?: string;
}

export const ThemeToggle: React.FC<LinearThemeToggleProps> = ({
  theme,
  onToggle,
  className = '',
}) => {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-label="Toggle theme"
      title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
      className={`size-7 p-0 shrink-0 inline-flex items-center justify-center rounded-[6px] border border-transparent bg-transparent text-muted hover:text-fg hover:bg-hover transition-colors duration-150 ease-out cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-text ${className}`}
    >
      {theme === 'dark' ? (
        <Sun className="size-3.5 shrink-0" aria-hidden="true" />
      ) : (
        <Moon className="size-3.5 shrink-0" aria-hidden="true" />
      )}
    </button>
  );
};
