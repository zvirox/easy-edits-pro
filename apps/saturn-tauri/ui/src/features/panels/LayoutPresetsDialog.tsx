import { useEffect, useState } from "react";
import { GlassDialog } from "../../motion/GlassDialog";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import { workspacePresets, type WorkspacePreset } from "./presets.ts";
import type { CustomWorkspacePreset } from "./presetStore.ts";

export type AnyWorkspacePreset = WorkspacePreset | CustomWorkspacePreset;

export interface LayoutPresetsDialogProps {
  open: boolean;
  onClose: () => void;
  activeWorkspace: string;
  customPresets: CustomWorkspacePreset[];
  onApply: (preset: AnyWorkspacePreset) => void;
  onSaveCurrent: (label: string) => void;
  onRename: (id: string, label: string) => void;
  onDelete: (id: string) => void;
  onResetActive: () => void;
}

/**
 * Workspace manager for floating-panel layouts: built-in presets, saved
 * layouts, rename/delete, and a reset for the active workspace.
 */
export function LayoutPresetsDialog({
  open,
  onClose,
  activeWorkspace,
  customPresets,
  onApply,
  onSaveCurrent,
  onRename,
  onDelete,
  onResetActive,
}: LayoutPresetsDialogProps) {
  const [label, setLabel] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingLabel, setEditingLabel] = useState("");

  useEffect(() => {
    if (!open) {
      setLabel("");
      setEditingId(null);
      setEditingLabel("");
    }
  }, [open]);

  const save = () => {
    if (!label.trim()) return;
    onSaveCurrent(label.trim());
    setLabel("");
  };

  const commitRename = () => {
    if (editingId && editingLabel.trim()) onRename(editingId, editingLabel.trim());
    setEditingId(null);
    setEditingLabel("");
  };

  const rows = (presets: readonly AnyWorkspacePreset[], custom: boolean) =>
    presets.map((preset) => (
      <div key={preset.id} className="layout-preset-row">
        <div className="layout-preset-copy">
          {editingId === preset.id ? (
            <input
              autoFocus
              value={editingLabel}
              aria-label="Preset name"
              onChange={(event) => setEditingLabel(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") commitRename();
                if (event.key === "Escape") setEditingId(null);
              }}
            />
          ) : (
            <strong>{preset.label}</strong>
          )}
          <span>{preset.workspace === activeWorkspace ? "This workspace" : preset.workspace}</span>
        </div>
        <div className="layout-preset-actions">
          <InteriorButton type="button" onClick={() => onApply(preset)}>
            Apply
          </InteriorButton>
          {custom ? (
            <>
              <InteriorButton
                type="button"
                onClick={() => {
                  setEditingId(preset.id);
                  setEditingLabel(preset.label);
                }}
              >
                Rename
              </InteriorButton>
              <InteriorButton
                type="button"
                className="layout-preset-delete"
                onClick={() => onDelete(preset.id)}
              >
                Delete
              </InteriorButton>
            </>
          ) : null}
        </div>
      </div>
    ));

  return (
    <GlassDialog
      open={open}
      cardClassName="modal-dialog-card glass layout-presets-dialog"
      onBackdropClick={onClose}
      cardProps={{
        role: "dialog",
        "aria-modal": true,
        "aria-labelledby": "layout-presets-title",
        onClick: (event) => event.stopPropagation(),
      }}
    >
      <div className="shortcuts-header">
        <div>
          <h2 id="layout-presets-title">Layout Presets</h2>
          <p>
            Workspaces are floating-panel layouts. Save the current arrangement,
            or restore a built-in preset.
          </p>
        </div>
        <InteriorButton type="button" className="close-btn" onClick={onClose} aria-label="Close">
          <Icon name="close" />
        </InteriorButton>
      </div>

      <label className="layout-preset-save">
        <span>Save current layout</span>
        <div>
          <input
            value={label}
            placeholder="Preset name"
            aria-label="New preset name"
            onChange={(event) => setLabel(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") save();
            }}
          />
          <InteriorButton type="button" onClick={save} disabled={!label.trim()}>
            Save
          </InteriorButton>
        </div>
      </label>

      <div className="layout-preset-list">
        <section aria-label="Saved presets">
          <h3>Saved</h3>
          {customPresets.length ? (
            rows(customPresets, true)
          ) : (
            <p className="layout-preset-empty">
              No saved layouts yet. Arrange the panels, then save them here.
            </p>
          )}
        </section>
        <section aria-label="Built-in presets">
          <h3>Built in</h3>
          {rows(workspacePresets, false)}
        </section>
      </div>

      <footer className="layout-preset-footer">
        <InteriorButton type="button" onClick={onResetActive}>
          Reset {activeWorkspace} layout
        </InteriorButton>
      </footer>
    </GlassDialog>
  );
}
