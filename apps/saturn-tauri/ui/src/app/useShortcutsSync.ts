import { useEffect, useRef, useState } from "react";
import { runCommand } from "./ProjectProvider";

const STORAGE_KEY = "easy-edits-pro.shortcuts.v1";

/**
 * Loads the keyboard-shortcut overrides (localStorage cache hydrated from
 * settings.json) and keeps every change persisted back to the engine.
 * Extracted from App.tsx to keep the component under the source-size limit.
 */
export function useShortcutsSync() {
  const hydrated = useRef(false);
  const [shortcutOverrides, setShortcutOverrides] = useState<Record<string, string>>(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      const parsed = raw ? JSON.parse(raw) : null;
      return parsed && typeof parsed === "object" ? (parsed as Record<string, string>) : {};
    } catch {
      return {};
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(shortcutOverrides));
    } catch {}
    if (!hydrated.current) return;
    void runCommand("shortcuts.set", { shortcuts: shortcutOverrides }).catch(() => {});
  }, [shortcutOverrides]);

  useEffect(() => {
    let active = true;
    void runCommand<{ shortcuts: Record<string, unknown> }>("shortcuts.list")
      .then(({ shortcuts }) => {
        if (!active) return;
        const overrides = Object.fromEntries(
          Object.entries(shortcuts ?? {}).filter(([, value]) => typeof value === "string"),
        ) as Record<string, string>;
        if (Object.keys(overrides).length) {
          setShortcutOverrides(overrides);
        } else if (Object.keys(shortcutOverrides).length) {
          // Migrate the localStorage cache into settings.json once.
          void runCommand("shortcuts.set", { shortcuts: shortcutOverrides }).catch(() => {});
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
  // eslint-disable-next-line react-hooks/exhaustive-deps -- hydration runs once; overrides synced above
  return { shortcutOverrides, setShortcutOverrides };
}
