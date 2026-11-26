// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useEffect, useState } from 'react';
import { Minus, Square, Copy, X } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeToggle } from './ThemeToggle';
import { UI_THEME_STORAGE_KEY } from '../../features/preferences/preferences';

export interface TitleBarProps {
  title?: string;
  version?: string;
  icon?: React.ReactNode;
  className?: string;
  onThemeToggle?: (theme: 'dark' | 'light') => void;
  children?: React.ReactNode;
}

function defaultVersionTag(): string {
  try {
    const v = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.1.3-beta.3';
    return v.startsWith('v') ? v : `v${v}`;
  } catch {
    return 'v0.1.3-beta.3';
  }
}

export const TitleBar: React.FC<TitleBarProps> = ({
  title = 'Easy Edits Pro',
  version = defaultVersionTag(),
  icon,
  className = '',
  onThemeToggle,
  children,
}) => {
  const [isMaximized, setIsMaximized] = useState(false);
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  useEffect(() => {
    const syncTheme = () => {
      const currentTheme =
        (document.documentElement.getAttribute('data-theme') as 'dark' | 'light') ||
        (document.documentElement.classList.contains('light') ? 'light' : 'dark');
      setTheme(currentTheme);
    };
    syncTheme();

    const observer = new MutationObserver(syncTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    });

    let unlistenPromise: Promise<() => void> | null = null;
    try {
      if (typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)) {
        const appWindow = getCurrentWindow();
        appWindow.isMaximized().then(setIsMaximized).catch(() => {});
        unlistenPromise = appWindow.onResized(() => {
          appWindow.isMaximized().then(setIsMaximized).catch(() => {});
        });
      }
    } catch {
      // Running outside Tauri runtime (browser/Playwright)
    }

    return () => {
      observer.disconnect();
      if (unlistenPromise) {
        unlistenPromise.then((fn) => fn()).catch(() => {});
      }
    };
  }, []);

  const handleMinimize = async () => {
    try {
      await getCurrentWindow().minimize();
    } catch {
      // fallback outside Tauri
    }
  };

  const handleToggleMaximize = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.toggleMaximize();
      setIsMaximized(await appWindow.isMaximized());
    } catch {
      setIsMaximized((prev) => !prev);
    }
  };

  const handleClose = async () => {
    try {
      await getCurrentWindow().close();
    } catch {
      // fallback outside Tauri
    }
  };

  const toggleTheme = () => {
    const nextTheme: 'dark' | 'light' = theme === 'dark' ? 'light' : 'dark';
    setTheme(nextTheme);
    document.documentElement.setAttribute('data-theme', nextTheme);
    document.documentElement.classList.toggle('dark', nextTheme === 'dark');
    document.documentElement.classList.toggle('light', nextTheme === 'light');
    try {
      localStorage.setItem(UI_THEME_STORAGE_KEY, nextTheme);
    } catch {}
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) {
      const bg = getComputedStyle(document.documentElement).getPropertyValue('--bg-app').trim();
      if (bg) meta.setAttribute('content', bg);
    }
    window.dispatchEvent(new CustomEvent('theme-changed', { detail: { theme: nextTheme } }));
    onThemeToggle?.(nextTheme);
  };

  const logoNode = icon ?? (
    <img
      src="/eep-icon.png"
      alt=""
      width={20}
      height={20}
      className="size-5 shrink-0 rounded-[4px] object-contain"
    />
  );

  return (
    <div
      role="region"
      aria-label="Window Title Bar"
      data-tauri-drag-region
      className={`h-10 w-full select-none bg-app border-b border-border flex items-center justify-between px-3 text-fg shrink-0 z-50 ${className}`}
    >
      {/* Left: 20px app logo, app name, version tag */}
      <div data-tauri-drag-region className="flex items-center gap-2 shrink-0">
        {logoNode}
        <span className="text-[13px] font-semibold tracking-tight text-fg">
          {title}
        </span>
        {version && (
          <span className="px-1.5 py-0.5 text-[11px] leading-none font-mono rounded-[4px] bg-surface-2 border border-border text-muted select-none">
            {version}
          </span>
        )}
      </div>

      {/* Center: empty drag region, double-click toggles maximize */}
      <div
        data-tauri-drag-region
        onDoubleClick={handleToggleMaximize}
        className="flex-1 h-full min-w-8 cursor-default"
      />

      {/* Right: theme toggle, divider, minimize, maximize/restore, close */}
      <div className="flex items-center gap-1 shrink-0">
        {children}
        {children && <div className="w-px h-3.5 bg-border mx-1" />}
        <ThemeToggle theme={theme} onToggle={toggleTheme} />
        <div className="w-px h-3.5 bg-border mx-1" aria-hidden="true" />
        <button
          type="button"
          onClick={handleMinimize}
          aria-label="Minimize window"
          title="Minimize"
          className="size-7 p-0 shrink-0 inline-flex items-center justify-center rounded-[6px] border border-transparent bg-transparent text-muted hover:text-fg hover:bg-hover transition-colors duration-150 ease-out cursor-pointer"
        >
          <Minus className="size-3.5 shrink-0" aria-hidden="true" />
        </button>
        <button
          type="button"
          onClick={handleToggleMaximize}
          aria-label={isMaximized ? 'Restore window' : 'Maximize window'}
          title={isMaximized ? 'Restore' : 'Maximize'}
          className="size-7 p-0 shrink-0 inline-flex items-center justify-center rounded-[6px] border border-transparent bg-transparent text-muted hover:text-fg hover:bg-hover transition-colors duration-150 ease-out cursor-pointer"
        >
          {isMaximized ? (
            <Copy className="size-3 shrink-0" aria-hidden="true" />
          ) : (
            <Square className="size-3 shrink-0" aria-hidden="true" />
          )}
        </button>
        <button
          type="button"
          onClick={handleClose}
          aria-label="Close window"
          title="Close"
          className="size-7 p-0 shrink-0 inline-flex items-center justify-center rounded-[6px] border border-transparent bg-transparent text-muted hover:bg-close-hover hover:text-fg transition-colors duration-150 ease-out cursor-pointer"
        >
          <X className="size-3.5 shrink-0" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
};
