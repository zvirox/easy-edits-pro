// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useCallback, useEffect, useRef, useState } from "react";
import { runCommand } from "../../app/ProjectProvider";
import { setReducedMotionOverride } from "../../motion/variants";
import {
  DEFAULT_PREFERENCES,
  loadPreferences,
  savePreferences,
  UI_THEME_STORAGE_KEY,
  type Preferences,
} from "./preferences";

export function useAppPreferences() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const hydrated = useRef(false);
  const theme = preferences.theme;

  const setTheme = useCallback((next: Preferences["theme"]) => {
    setPreferences((current) => ({ ...current, theme: next }));
  }, []);

  const toggleTheme = useCallback(
    () => setTheme(theme === "dark" ? "light" : "dark"),
    [setTheme, theme],
  );

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.documentElement.classList.toggle("light", theme === "light");
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) {
      const bg = getComputedStyle(document.documentElement).getPropertyValue("--bg-app").trim();
      if (bg) meta.setAttribute("content", bg);
    }
  }, [theme]);

  useEffect(() => {
    const onThemeChanged = (event: Event) => {
      const next = (event as CustomEvent<{ theme?: Preferences["theme"] }>).detail?.theme;
      if (next === "dark" || next === "light") {
        setPreferences((current) => (current.theme === next ? current : { ...current, theme: next }));
      }
    };
    window.addEventListener("theme-changed", onThemeChanged);
    return () => window.removeEventListener("theme-changed", onThemeChanged);
  }, []);

  useEffect(() => {
    savePreferences(preferences);
    if (!hydrated.current) return;
    void runCommand("prefs.set", { values: preferences }).catch(() => {});
  }, [preferences]);

  useEffect(() => {
    let active = true;
    void runCommand<{ prefs: Record<string, unknown> }>("prefs.get")
      .then(({ prefs }) => {
        if (!active) return;
        const allowed = new Set(Object.keys(DEFAULT_PREFERENCES));
        const stored = Object.fromEntries(
          Object.entries(prefs ?? {}).filter(([key]) => allowed.has(key)),
        ) as Partial<Preferences>;
        if (Object.keys(stored).length) {
          const cachedTheme =
            typeof localStorage !== "undefined" ? localStorage.getItem(UI_THEME_STORAGE_KEY) : null;
          setPreferences((current) => ({
            ...current,
            ...stored,
            theme:
              cachedTheme === "dark" || cachedTheme === "light" ? cachedTheme : current.theme,
            glass: "off",
          }));
        }
      })
      .catch(() => {})
      .finally(() => {
        hydrated.current = true;
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    setReducedMotionOverride(preferences.reducedMotion);
  }, [preferences.reducedMotion]);

  useEffect(() => {
    document.documentElement.setAttribute("data-glass", "off");
  }, []);

  return { preferences, setPreferences, theme, setTheme, toggleTheme };
}
