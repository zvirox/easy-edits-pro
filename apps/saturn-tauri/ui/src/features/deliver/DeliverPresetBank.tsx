import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import { runCommand } from "../../app/ProjectProvider";

export interface EngineExportSettings {
  format?: string;
  resolution?: string;
  frame_rate?: string;
  use_hardware?: boolean;
  use_mark_range?: boolean;
}

interface DeliverPresetBankProps {
  disabled: boolean;
  /** Current workspace settings; Save As stores exactly these. */
  format: "mp4" | "webm";
  resolution: string;
  frameRate: string;
  useHardware: boolean;
  useMarkRange: boolean;
  /** Apply an engine preset's settings to the workspace controls. */
  applyPreset: (settings: EngineExportSettings) => void;
  onNotice: (message: string) => void;
  onQuickExport: () => Promise<void> | void;
}

/**
 * Engine-backed export preset bank for the Deliver workspace. The list, the
 * resolution math, and the file import/export all go through the export.*
 * commands, so the CLI, MCP bridge, and this panel share one preset store.
 */
export function DeliverPresetBank({
  disabled,
  format,
  resolution,
  frameRate,
  useHardware,
  useMarkRange,
  applyPreset,
  onNotice,
  onQuickExport,
}: DeliverPresetBankProps) {
  const [presets, setPresets] = useState<string[]>([]);
  const [presetChoice, setPresetChoice] = useState("source");
  const [summary, setSummary] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await runCommand<{ presets: Record<string, unknown> }>(
        "export.presets",
        {},
      );
      const names = Object.keys(result.presets ?? {}).sort((a, b) =>
        a === "source" ? -1 : b === "source" ? 1 : a.localeCompare(b),
      );
      setPresets(names);
      setPresetChoice((current) => (names.includes(current) ? current : names[0] ?? "source"));
    } catch (error) {
      onNotice(String(error));
    }
  }, [onNotice]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Live resolve summary: what the current workspace controls will render.
  useEffect(() => {
    let active = true;
    void runCommand<Record<string, unknown>>("export.resolve", {
      format,
      resolution,
      frame_rate: frameRate,
      use_hardware: useHardware,
      use_mark_range: useMarkRange,
    })
      .then((resolved) => {
        if (!active) return;
        const rate = `${resolved.frame_rate_numerator}/${resolved.frame_rate_denominator}`;
        setSummary(
          `${resolved.width}x${resolved.height} at ${rate}, ${String(resolved.format)}${
            resolved.use_mark_range ? ", marked range" : ""
          }${resolved.use_hardware ? ", hardware encode" : ""}`,
        );
      })
      .catch((error) => {
        if (active) setSummary(String(error));
      });
    return () => {
      active = false;
    };
  }, [format, resolution, frameRate, useHardware, useMarkRange]);

  const apply = () =>
    void runCommand<{ settings: EngineExportSettings | null }>(
      "export.presets.get",
      { name: presetChoice },
    )
      .then((result) => {
        if (!result.settings) {
          onNotice(`Preset "${presetChoice}" has no settings`);
          return;
        }
        applyPreset(result.settings);
        onNotice(`Applied preset "${presetChoice}"`);
      })
      .catch((error) => onNotice(String(error)));

  const saveAs = () => {
    const name = window.prompt("Save current export settings as preset", "");
    if (!name?.trim()) return;
    void runCommand("export.presets.save", {
      name: name.trim(),
      settings: {
        format,
        resolution,
        frame_rate: frameRate,
        use_hardware: useHardware,
        use_mark_range: useMarkRange,
      },
    })
      .then(() => {
        setPresetChoice(name.trim());
        void refresh();
        onNotice(`Preset saved: ${name.trim()}`);
      })
      .catch((error) => onNotice(String(error)));
  };

  const remove = () =>
    void runCommand("export.presets.delete", { name: presetChoice })
      .then(() => {
        void refresh();
        onNotice(`Preset deleted: ${presetChoice}`);
      })
      .catch((error) => onNotice(String(error)));

  const favorite = () =>
    void runCommand<{ favorite: boolean }>("export.presets.favorite", { name: presetChoice })
      .then((result) =>
        onNotice(
          `${presetChoice} is ${result.favorite ? "now a favorite" : "no longer a favorite"}`,
        ),
      )
      .catch((error) => onNotice(String(error)));

  const importFile = () =>
    void open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    })
      .then((path) => {
        if (typeof path !== "string" || !path) return;
        return invoke<string>("read_text_file", { path }).then((content) =>
          runCommand<{ imported: number }>("export.presets.import", {
            presets: JSON.parse(content),
          }),
        );
      })
      .then((result) => {
        if (!result) return;
        void refresh();
        onNotice(`Imported ${result.imported} export preset(s)`);
      })
      .catch((error) => onNotice(String(error)));

  const exportFile = () =>
    void save({
      title: "Export export presets",
      filters: [{ name: "JSON", extensions: ["json"] }],
    })
      .then((path) => {
        if (typeof path !== "string" || !path) return;
        void runCommand<{ exported: number }>("export.presets.export", { path }).then((result) =>
          onNotice(`Exported ${result.exported} preset(s) to ${path}`),
        );
      })
      .catch((error) => onNotice(String(error)));

  return (
    <div className="export-presets-shelf">
      <div className="export-presets-shelf-header">
        <strong>ENGINE PRESET BANK</strong>
        <small data-testid="export-resolve-summary">
          Resolves to {summary ?? "checking settings..."}
        </small>
      </div>
      <div className="export-preset-bank-controls" style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center" }}>
        <select
          value={presetChoice}
          disabled={disabled}
          onChange={(event) => setPresetChoice(event.target.value)}
          aria-label="Engine export preset"
          style={{ flex: 1, minWidth: 140 }}
        >
          {presets.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <InteriorButton type="button" disabled={disabled} onClick={apply} title="Load this preset into the settings form (export.presets.get)">
          Apply
        </InteriorButton>
        <InteriorButton type="button" disabled={disabled} onClick={favorite} title="Mark this preset as a favorite (export.presets.favorite)">
          Favorite
        </InteriorButton>
        <InteriorButton type="button" disabled={disabled} onClick={remove} title="Delete this user preset (export.presets.delete)">
          <Icon name="close" />
        </InteriorButton>
        <InteriorButton type="button" disabled={disabled} onClick={saveAs} title="Save the current settings under a new name (export.presets.save)">
          Save As
        </InteriorButton>
        <InteriorButton type="button" disabled={disabled} onClick={importFile} title="Import presets from a JSON file (export.presets.import)">
          Import
        </InteriorButton>
        <InteriorButton type="button" disabled={disabled} onClick={exportFile} title="Write all presets to a JSON file (export.presets.export)">
          Export
        </InteriorButton>
      </div>
      <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
        <InteriorButton
          type="button"
          className="batch-queue-btn"
          disabled={disabled}
          onClick={() => void onQuickExport()}
          title="Queue a full-timeline export with the factory source preset (export.quick)"
        >
          Quick Export
        </InteriorButton>
      </div>
    </div>
  );
}
