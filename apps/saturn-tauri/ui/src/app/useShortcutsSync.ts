import { useEffect, useMemo, useRef, useState } from "react";
import { runCommand } from "./ProjectProvider";

const STORAGE_KEY = "easy-edits-pro.shortcuts.v1";

export interface ShortcutAuditInfo {
  total: number;
  empty: number;
  conflicts: number;
}

/** Engine-backed shortcut operations (settings.json scoped). */
export interface ShortcutsEngineOps {
  audit: () => Promise<ShortcutAuditInfo>;
  forKey: (key: string) => Promise<string[]>;
  get: (commandId: string) => Promise<string | null>;
  presets: () => Promise<string[]>;
  loadPreset: (name: string) => Promise<void>;
  savePreset: (name: string) => Promise<unknown>;
  deletePreset: (name: string) => Promise<void>;
  clear: (commandId: string) => Promise<void>;
  resolve: (key: string, keep: string) => Promise<void>;
  exportShortcuts: (path: string) => Promise<number>;
  undoChange: () => Promise<void>;
  redoChange: () => Promise<void>;
}

/**
 * Loads the keyboard-shortcut overrides (localStorage cache hydrated from
 * settings.json) and keeps every change persisted back to the engine.
 * Extracted from App.tsx to keep the component under the source-size limit.
 */
export function useShortcutsSync() {
  const hydrated = useRef(false);
  /** Set before an engine-driven override replacement to skip the write-back. */
  const skipWrite = useRef(false);
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
    if (!hydrated.current || skipWrite.current) {
      skipWrite.current = false;
      return;
    }
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
  const engineOps: ShortcutsEngineOps = useMemo(() => {
    const applyEngineMap = (shortcuts: Record<string, unknown> | undefined) => {
      const next = Object.fromEntries(
        Object.entries(shortcuts ?? {}).filter(([, value]) => typeof value === "string"),
      ) as Record<string, string>;
      skipWrite.current = true;
      setShortcutOverrides(next);
    };
    const refresh = async () => {
      const { shortcuts } = await runCommand<{ shortcuts: Record<string, unknown> }>(
        "shortcuts.list",
      );
      applyEngineMap(shortcuts);
    };
    return {
      audit: () =>
        runCommand<{ total: number; empty: number; conflicts: number }>("shortcuts.audit", {}),
      forKey: async (key: string) => {
        const result = await runCommand<{ commands: string[] }>("shortcuts.for_key", { key });
        return result.commands;
      },
      get: async (commandId: string) => {
        const result = await runCommand<{ shortcut: unknown }>("shortcuts.get", { command_id: commandId });
        return typeof result.shortcut === "string" ? result.shortcut : null;
      },
      presets: async () => {
        const result = await runCommand<{ presets: Record<string, unknown> }>(
          "shortcuts.presets",
          {},
        );
        return Object.keys(result.presets ?? {}).sort((a, b) =>
          a === "default" ? -1 : b === "default" ? 1 : a.localeCompare(b),
        );
      },
      loadPreset: async (name: string) => {
        const result = await runCommand<{ shortcuts: Record<string, unknown> }>(
          "shortcuts.load_preset",
          { name },
        );
        applyEngineMap(result.shortcuts);
      },
      savePreset: (name: string) => runCommand("shortcuts.save_preset", { name }),
      deletePreset: async (name: string) => {
        await runCommand("shortcuts.delete_preset", { name });
      },
      clear: async (commandId: string) => {
        await runCommand("shortcuts.clear", { command_id: commandId });
        await refresh();
      },
      resolve: async (key: string, keep: string) => {
        await runCommand("shortcuts.resolve", { key, keep });
        await refresh();
      },
      exportShortcuts: async (path: string) => {
        const result = await runCommand<{ exported: number }>("shortcuts.export", { path });
        return result.exported;
      },
      undoChange: async () => {
        const result = await runCommand<{ shortcuts: Record<string, unknown> }>(
          "shortcuts.undo",
          {},
        );
        applyEngineMap(result.shortcuts);
      },
      redoChange: async () => {
        const result = await runCommand<{ shortcuts: Record<string, unknown> }>(
          "shortcuts.redo",
          {},
        );
        applyEngineMap(result.shortcuts);
      },
    };
  }, []);

  return { shortcutOverrides, setShortcutOverrides, engineOps };
}
