import { useEffect, useMemo, useRef, useState } from "react";
import { GlassDialog } from "../../motion/GlassDialog";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import type { CommandSpec } from "../../types";
import {
  comboFromEvent,
  findShortcutConflict,
  formatShortcut,
  parseShortcutImport,
  serializeShortcuts,
} from "./shortcuts.ts";

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
  onImport: (overrides: Record<string, string>) => void;
  onResetAll: () => void;
}

type Row = {
  id: string;
  label: string;
  group: string;
  shortcut: string;
  editable: boolean;
};

/**
 * Searchable shortcut reference with conflict-checked rebinding for menu
 * commands. Changes dispatch `shortcuts.set` to the engine (settings.json)
 * through `useShortcutsSync`, which mirrors the overrides in localStorage.
 */
export function ShortcutsDialog({
  open,
  onClose,
  commands,
  tools,
  overrides,
  onChange,
  onImport,
  onResetAll,
}: ShortcutsDialogProps) {
  const [query, setQuery] = useState("");
  const [listening, setListening] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [transferOpen, setTransferOpen] = useState(false);
  const [transferText, setTransferText] = useState("");
  const [copied, setCopied] = useState(false);
  const transferRef = useRef<HTMLTextAreaElement>(null);

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

  const editableIds = useMemo(
    () => new Set(rows.filter((row) => row.editable).map((row) => row.id)),
    [rows],
  );

  const openTransfer = () => {
    setTransferText(serializeShortcuts(overrides));
    setCopied(false);
    setTransferOpen((value) => !value);
  };

  const applyTransfer = () => {
    const result = parseShortcutImport(transferText, editableIds);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    onImport(result.overrides);
    setError(null);
    setTransferOpen(false);
  };

  const copyTransfer = async () => {
    try {
      await navigator.clipboard.writeText(transferText);
      setCopied(true);
    } catch {
      transferRef.current?.select();
      setError("Clipboard is unavailable; the JSON is selected so you can copy it.");
    }
  };

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
      setTransferOpen(false);
      setCopied(false);
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
        const conflict = findShortcutConflict(rows, listening, combo);
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
                      {listening === row.id
                        ? "Press keys…"
                        : formatShortcut(row.shortcut) || "Unassigned"}
                    </InteriorButton>
                  ) : (
                    <span
                      className="shortcuts-key static"
                      title="Timeline tool keys ship with the build"
                    >
                      {formatShortcut(row.shortcut)}
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

      {transferOpen ? (
        <section className="shortcuts-transfer" aria-label="Import or export shortcuts">
          <div className="shortcuts-transfer-head">
            <h3>Import / Export</h3>
            <span>JSON keyed by command id. Paste a file’s contents to import.</span>
          </div>
          <textarea
            ref={transferRef}
            value={transferText}
            spellCheck={false}
            aria-label="Shortcut JSON"
            onChange={(event) => {
              setTransferText(event.target.value);
              setCopied(false);
            }}
          />
          <div className="shortcuts-transfer-actions">
            <InteriorButton type="button" onClick={() => void copyTransfer()}>
              {copied ? "Copied" : "Copy JSON"}
            </InteriorButton>
            <InteriorButton type="button" onClick={applyTransfer}>
              Apply pasted JSON
            </InteriorButton>
          </div>
        </section>
      ) : null}

      <footer className="shortcuts-footer">
        <span>{visible.length} entries</span>
        <InteriorButton type="button" className="shortcuts-import" onClick={openTransfer}>
          {transferOpen ? "Hide import / export" : "Import / export"}
        </InteriorButton>
        <InteriorButton type="button" className="shortcuts-reset" onClick={onResetAll}>
          Reset all to defaults
        </InteriorButton>
      </footer>
    </GlassDialog>
  );
}
