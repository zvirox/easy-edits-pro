import { useEffect, useMemo, useState } from "react";
import { GlassDialog } from "../../motion/GlassDialog";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import type { CommandSpec } from "../../types";

export interface ToolShortcut {
  id: string;
  label: string;
  shortcut: string;
}

export interface ShortcutsDialogProps {
  open: boolean;
  onClose: () => void;
  commands: CommandSpec[];
  tools: ToolShortcut[];
  /** Command id → shortcut. An empty string removes the binding. */
  overrides: Record<string, string>;
  onChange: (id: string, shortcut: string) => void;
  onResetAll: () => void;
}

type Row = {
  id: string;
  label: string;
  group: string;
  shortcut: string;
  editable: boolean;
};

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "CapsLock", "Tab"]);

function comboFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key) || event.key === "Escape") return null;
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  const key = event.key === " " ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key;
  parts.push(key);
  return parts.join("+");
}

/**
 * Searchable shortcut reference with conflict-checked rebinding for menu
 * commands. Overrides live in localStorage until the preferences engine lands.
 */
export function ShortcutsDialog({
  open,
  onClose,
  commands,
  tools,
  overrides,
  onChange,
  onResetAll,
}: ShortcutsDialogProps) {
  const [query, setQuery] = useState("");
  const [listening, setListening] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const rows = useMemo<Row[]>(() => {
    const commandRows = commands
      .filter((command) => command.shipped)
      .map((command) => ({
        id: command.id,
        label: command.label,
        group: command.menu.join(" › ") || "Commands",
        shortcut:
          overrides[command.id] !== undefined ? overrides[command.id] : command.shortcut ?? "",
        editable: true,
      }));
    const toolRows = tools.map((tool) => ({
      id: `tool.${tool.id}`,
      label: tool.label,
      group: "Timeline tools",
      shortcut: tool.shortcut,
      editable: false,
    }));
    return [...commandRows, ...toolRows];
  }, [commands, tools, overrides]);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter(
      (row) =>
        row.label.toLowerCase().includes(needle) ||
        row.group.toLowerCase().includes(needle) ||
        row.shortcut.toLowerCase().includes(needle),
    );
  }, [rows, query]);

  const grouped = useMemo(() => {
    const groups = new Map<string, Row[]>();
    for (const row of visible) {
      const bucket = groups.get(row.group);
      if (bucket) bucket.push(row);
      else groups.set(row.group, [row]);
    }
    return [...groups.entries()];
  }, [visible]);

  useEffect(() => {
    if (!open) {
      setListening(null);
      setError(null);
      setQuery("");
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (listening) {
        event.preventDefault();
        event.stopPropagation();
        const combo = comboFromEvent(event);
        if (event.key === "Escape") {
          setListening(null);
          setError(null);
          return;
        }
        if (!combo) return;
        const conflict = rows.find(
          (row) => row.id !== listening && row.shortcut.toLowerCase() === combo.toLowerCase(),
        );
        if (conflict) {
          setError(`${combo} is already assigned to ${conflict.label}.`);
          return;
        }
        onChange(listening, combo);
        setListening(null);
        setError(null);
        return;
      }
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [open, listening, rows, onChange, onClose]);

  return (
    <GlassDialog
      open={open}
      cardClassName="modal-dialog-card glass shortcuts-dialog"
      onBackdropClick={onClose}
      cardProps={{
        role: "dialog",
        "aria-modal": true,
        "aria-labelledby": "shortcuts-dialog-title",
        onClick: (event) => event.stopPropagation(),
      }}
    >
      <div className="shortcuts-header">
        <div>
          <h2 id="shortcuts-dialog-title">Keyboard Shortcuts</h2>
          <p>
            Click a shortcut to rebind it. Escape cancels. Changes are stored on this
            machine.
          </p>
        </div>
        <InteriorButton type="button" className="close-btn" onClick={onClose} aria-label="Close">
          <Icon name="close" />
        </InteriorButton>
      </div>

      <label className="shortcuts-search">
        <Icon name="search" />
        <input
          autoFocus
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search commands, menus, or keys…"
          aria-label="Search shortcuts"
        />
      </label>

      {error ? <p className="shortcuts-error" role="alert">{error}</p> : null}

      <div className="shortcuts-list">
        {grouped.length ? (
          grouped.map(([group, entries]) => (
            <section key={group} className="shortcuts-group" aria-label={group}>
              <h3>{group}</h3>
              {entries.map((row) => (
                <div key={row.id} className="shortcuts-row">
                  <span className="shortcuts-row-label">{row.label}</span>
                  {row.editable ? (
                    <InteriorButton
                      type="button"
                      className={`shortcuts-key ${listening === row.id ? "listening" : ""}`}
                      onClick={() => {
                        setListening(row.id);
                        setError(null);
                      }}
                      aria-label={`Change shortcut for ${row.label}`}
                    >
                      {listening === row.id ? "Press keys…" : row.shortcut || "Unassigned"}
                    </InteriorButton>
                  ) : (
                    <span
                      className="shortcuts-key static"
                      title="Timeline tool keys ship with the build"
                    >
                      {row.shortcut}
                    </span>
                  )}
                </div>
              ))}
            </section>
          ))
        ) : (
          <p className="shortcuts-empty">No shortcuts match “{query}”.</p>
        )}
      </div>

      <footer className="shortcuts-footer">
        <span>{visible.length} entries</span>
        <InteriorButton type="button" className="shortcuts-reset" onClick={onResetAll}>
          Reset all to defaults
        </InteriorButton>
      </footer>
    </GlassDialog>
  );
}
