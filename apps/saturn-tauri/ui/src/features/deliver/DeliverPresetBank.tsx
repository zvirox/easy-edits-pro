// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import { runCommand } from "../../app/ProjectProvider";
import { grantWrite } from "../../app/writeGrants.ts";
import type { ProjectDocument } from "../../app/projectTypes";

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
  project?: ProjectDocument["project"] | null;
  /** Apply an engine preset's settings to the workspace controls. */
  applyPreset: (settings: EngineExportSettings) => void;
  onNotice: (message: string) => void;
  onQuickExport: () => Promise<void> | void;
}

function formatResolveSummary(
  resolved: Record<string, unknown> | null | undefined,
  fallback: {
    format: "mp4" | "webm";
    resolution: string;
    frameRate: string;
    useHardware: boolean;
    useMarkRange: boolean;
    project?: ProjectDocument["project"] | null;
  },
): string {
  let width = typeof resolved?.width === "number" ? resolved.width : null;
  let height = typeof resolved?.height === "number" ? resolved.height : null;
  if (!width || !height) {
    if (fallback.resolution && fallback.resolution !== "source") {
      const parts = fallback.resolution.split("x").map(Number);
      if (parts[0] && parts[1]) {
        width = parts[0];
        height = parts[1];
      }
    }
    width ||= fallback.project?.settings.width ?? 1920;
    height ||= fallback.project?.settings.height ?? 1080;
  }

  let num = typeof resolved?.frame_rate_numerator === "number" ? resolved.frame_rate_numerator : null;
  let den = typeof resolved?.frame_rate_denominator === "number" ? resolved.frame_rate_denominator : null;
  if (!num || !den) {
    if (fallback.frameRate && fallback.frameRate !== "source") {
      const parts = fallback.frameRate.split("/").map(Number);
      if (parts[0] && parts[1]) {
        num = parts[0];
        den = parts[1];
      }
    }
    num ||= fallback.project?.settings.frame_rate.numerator ?? 30;
    den ||= fallback.project?.settings.frame_rate.denominator ?? 1;
  }

  const fmt = (typeof resolved?.format === "string" && resolved.format ? resolved.format : fallback.format) || "mp4";
  const isMarkRange = typeof resolved?.use_mark_range === "boolean" ? resolved.use_mark_range : fallback.useMarkRange;
  const isHardware = typeof resolved?.use_hardware === "boolean" ? resolved.use_hardware : fallback.useHardware;

  return `${width}x${height} at ${num}/${den}, ${fmt}${isMarkRange ? ", marked range" : ""}${isHardware ? ", hardware encode" : ""}`;
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
  project,
  applyPreset,
  onNotice,
  onQuickExport,
}: DeliverPresetBankProps) {
  const [presets, setPresets] = useState<string[]>([]);
  const [presetChoice, setPresetChoice] = useState("source");
  const [summary, setSummary] = useState<string>(() =>
    formatResolveSummary(null, { format, resolution, frameRate, useHardware, useMarkRange, project }),
  );

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
        setSummary(
          formatResolveSummary(resolved, { format, resolution, frameRate, useHardware, useMarkRange, project }),
        );
      })
      .catch(() => {
        if (!active) return;
        setSummary(
          formatResolveSummary(null, { format, resolution, frameRate, useHardware, useMarkRange, project }),
        );
      });
    return () => {
      active = false;
    };
  }, [format, resolution, frameRate, useHardware, useMarkRange, project]);

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
        void grantWrite(path)
          .then(() =>
            runCommand<{ exported: number }>("export.presets.export", { path }),
          )
          .then((result) => onNotice(`Exported ${result.exported} preset(s) to ${path}`))
          .catch((error) => onNotice(String(error)));
      })
      .catch((error) => onNotice(String(error)));

  const packageDelivery = () =>
    void save({
      title: "Create Delivery Package Folder",
      defaultPath: "delivery-package",
    })
      .then((path) => {
        if (typeof path !== "string" || !path) return;
        void grantWrite(path)
          .then(() =>
            runCommand<{
              verified?: boolean;
              verification?: { checked_files?: number };
            }>("delivery.package", { path }),
          )
          .then((result) => {
            const count = result.verification?.checked_files ?? 0;
            onNotice(`Packaged & verified ${count} delivery artifact(s) in ${path}`);
          })
          .catch((error) => onNotice(String(error)));
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
      <div className="export-preset-bank-actions">
        <InteriorButton
          type="button"
          className="quick-export-btn"
          disabled={disabled}
          onClick={() => void onQuickExport()}
          title="Queue a full-timeline export with the factory source preset (export.quick)"
        >
          ⚡ Quick Export
        </InteriorButton>
        <InteriorButton
          type="button"
          className="package-delivery-btn"
          disabled={disabled}
          onClick={packageDelivery}
          title="Create a checksum-verified TS5 handoff package with project, FCPXML, OTIO, QC report, change log, and provenance (delivery.package)"
        >
          📦 Package Delivery…
        </InteriorButton>
      </div>
    </div>
  );
}
