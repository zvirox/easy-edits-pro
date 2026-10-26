import { save } from "@tauri-apps/plugin-dialog";
import type { ShortcutsEngineOps } from "./useShortcutsSync";

export interface ShortcutMachineDeps {
  engineOps: ShortcutsEngineOps;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  setNotice: (message: string) => void;
}

/**
 * Machine entries for the shortcut settings commands: each one prompts for
 * its parameter (app-native prompt idiom) and reports the engine result as a
 * notice. The Shortcuts dialog exposes the same operations with widgets.
 * Exported as a factory so App.tsx stays under the source-size limit.
 */
export function createShortcutMachineCommand({
  engineOps,
  runCommand,
  setNotice,
}: ShortcutMachineDeps) {
  return async function runShortcutMachineCommand(id: string): Promise<void> {
    const report = (label: string, value: unknown) => {
      const detail = value === "" || value == null ? "" : ` ${JSON.stringify(value)}`;
      setNotice(`${label}${detail}`);
    };
    try {
      switch (id) {
        case "shortcuts.get": {
          const commandId = window.prompt("Command id", "playhead.play_pause");
          if (!commandId) return;
          const binding = await engineOps.get(commandId);
          setNotice(binding ? `${commandId}: ${binding}` : `${commandId}: no override`);
          return;
        }
        case "shortcuts.for_key": {
          const key = window.prompt("Key combo", "Ctrl+K");
          if (!key) return;
          const users = await engineOps.forKey(key);
          setNotice(users.length ? `${key}: ${users.join(", ")}` : `${key}: unbound`);
          return;
        }
        case "shortcuts.conflicts": {
          const result = await runCommand<{ conflicts: unknown[] }>(
            "shortcuts.conflicts",
            {},
          );
          report(
            result.conflicts.length
              ? `${result.conflicts.length} conflict(s):`
              : "No shortcut conflicts",
            result.conflicts.length ? result.conflicts : "",
          );
          return;
        }
        case "shortcuts.audit": {
          const info = await engineOps.audit();
          setNotice(
            `Shortcut audit: ${info.total} override(s), ${info.conflicts} conflict(s), ${info.empty} empty`,
          );
          return;
        }
        case "shortcuts.presets": {
          const names = await engineOps.presets();
          setNotice(`Shortcut presets: ${names.join(", ")}`);
          return;
        }
        case "shortcuts.save_preset": {
          const name = window.prompt("Save current overrides as preset", "My layout");
          if (!name?.trim()) return;
          await engineOps.savePreset(name.trim());
          setNotice(`Shortcut preset saved: ${name.trim()}`);
          return;
        }
        case "shortcuts.load_preset": {
          const name = window.prompt("Load shortcut preset", "default");
          if (!name?.trim()) return;
          await engineOps.loadPreset(name.trim());
          setNotice(`Shortcut preset loaded: ${name.trim()}`);
          return;
        }
        case "shortcuts.delete_preset": {
          const name = window.prompt("Delete shortcut preset", "");
          if (!name?.trim()) return;
          await engineOps.deletePreset(name.trim());
          setNotice(`Shortcut preset deleted: ${name.trim()}`);
          return;
        }
        case "shortcuts.clear": {
          const commandId = window.prompt("Command id to clear", "playhead.play_pause");
          if (!commandId) return;
          await engineOps.clear(commandId);
          setNotice(`Shortcut override cleared: ${commandId}`);
          return;
        }
        case "shortcuts.resolve": {
          const key = window.prompt("Key combo to resolve", "Ctrl+K");
          if (!key) return;
          const keep = window.prompt("Command that keeps the key", "");
          if (!keep) return;
          await engineOps.resolve(key, keep);
          setNotice(`Resolved ${key} to ${keep}`);
          return;
        }
        case "shortcuts.export": {
          const path = await save({
            title: "Export keyboard shortcuts",
            filters: [{ name: "JSON", extensions: ["json"] }],
          });
          if (typeof path !== "string" || !path) return;
          const count = await engineOps.exportShortcuts(path);
          setNotice(`Exported ${count} shortcut override(s) to ${path}`);
          return;
        }
        case "shortcuts.undo":
          await engineOps.undoChange();
          setNotice("Shortcut change undone");
          return;
        case "shortcuts.redo":
          await engineOps.redoChange();
          setNotice("Shortcut change redone");
          return;
        default:
          return;
      }
    } catch (error) {
      setNotice(String(error));
    }
  };
}
