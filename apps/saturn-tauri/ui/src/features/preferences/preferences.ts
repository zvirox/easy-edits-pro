// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
export type ThemeMode = "dark" | "light";
export type GlassMode = "full" | "lite" | "off";
export type TrackHeightPreference = "compact" | "normal" | "expanded";

export interface Preferences {
  theme: ThemeMode;
  glass: GlassMode;
  /** null follows the operating system setting. */
  reducedMotion: boolean | null;
  snap: boolean;
  trackHeight: TrackHeightPreference;
}

export const PREFERENCES_STORAGE_KEY = "easy-edits-pro.preferences.v1";
export const UI_THEME_STORAGE_KEY = "ui-theme";

export const DEFAULT_PREFERENCES: Preferences = {
  theme: "dark",
  glass: "off",
  reducedMotion: null,
  snap: true,
  trackHeight: "normal",
};

function systemTheme(): ThemeMode {
  try {
    if (typeof window !== "undefined" && window.matchMedia) {
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
  } catch {}
  return "dark";
}

function resolveInitialTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem(UI_THEME_STORAGE_KEY);
    if (saved === "light" || saved === "dark") return saved;
  } catch {}
  return systemTheme();
}

function isTrackHeight(value: unknown): value is TrackHeightPreference {
  return value === "compact" || value === "normal" || value === "expanded";
}

/** Glass is permanently off in the locked Linear UI. */
export function storedGlassMode(): GlassMode | null {
  return "off";
}

export function loadPreferences(): Preferences {
  let stored: Partial<Preferences> = {};
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (raw) stored = JSON.parse(raw) as Partial<Preferences>;
  } catch {
    stored = {};
  }
  return {
    theme: resolveInitialTheme(),
    glass: "off",
    reducedMotion:
      stored.reducedMotion === true || stored.reducedMotion === false
        ? stored.reducedMotion
        : null,
    snap: stored.snap !== false,
    trackHeight: isTrackHeight(stored.trackHeight) ? stored.trackHeight : "normal",
  };
}

export function savePreferences(preferences: Preferences): void {
  try {
    localStorage.setItem(UI_THEME_STORAGE_KEY, preferences.theme);
    localStorage.setItem(
      PREFERENCES_STORAGE_KEY,
      JSON.stringify({ ...preferences, glass: "off" }),
    );
  } catch {}
}
