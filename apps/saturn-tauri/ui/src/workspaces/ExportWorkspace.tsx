import type { Dispatch, SetStateAction, ReactNode } from "react";
import { Icon } from "../components/Icon";
import { StudioPanel } from "../components/StudioPanel";
import {
  type ProjectDocument,
  type MediaAsset,
  type HardwareEncoderInfo,
  type BatchQueueItem,
  resolutionPresets,
  exportPresets,
  exportFrameRateOptions,
} from "../types";
import { formatTime } from "../utils/formatters";

export interface ExportWorkspaceProps {
  project: ProjectDocument["project"] | null;
  batchQueue: BatchQueueItem[];
  setBatchQueue: Dispatch<SetStateAction<BatchQueueItem[]>>;
  queuePaused: boolean;
  toggleQueuePause: () => void | Promise<void>;
  deliverDestination: "render" | "interop";
  setDeliverDestination: (dest: "render" | "interop") => void;
  handleExportTimelineInterop: (format: "fcpxml" | "premiere_xml" | "edl") => Promise<void>;
  handleImportTimelineInterop: () => Promise<void>;
  isInteropBusy: boolean;
  interopContent: string;
  setNotice: (msg: string) => void;
  encoderInfo: HardwareEncoderInfo | null;
  selectedExportPreset: string;
  setSelectedExportPreset: (preset: string) => void;
  exportResolution: string;
  setExportResolution: (res: string) => void;
  exportFrameRate: string;
  setExportFrameRate: (fps: string) => void;
  exportFormat: "mp4" | "webm";
  setExportFormat: (fmt: "mp4" | "webm") => void;
  exportFileName: string;
  setExportFileName: (name: string) => void;
  exportOutputPath: string;
  setExportOutputPath: (path: string) => void;
  chooseExportDestination: () => Promise<unknown>;
  useHardwareEncoding: boolean;
  setUseHardwareEncoding: (val: boolean) => void;
  exportMarkedRange: boolean;
  setExportMarkedRange: (val: boolean) => void;
  invokeMenuCommand: (id: string) => Promise<unknown>;
  activeClip: { media?: MediaAsset | null } | null;
  selectedMedia: MediaAsset | null;
  renderPreview: (item: MediaAsset | null, isProgram?: boolean) => ReactNode;
  rendering: boolean;
  renderProgress: { fractionPercent: number; phase: string } | null;
  addExportToQueue: () => Promise<void> | void;
  exportProject: () => Promise<void> | void;
}

export function ExportWorkspace({
  project,
  batchQueue,
  setBatchQueue,
  queuePaused,
  toggleQueuePause,
  deliverDestination,
  setDeliverDestination,
  handleExportTimelineInterop,
  handleImportTimelineInterop,
  isInteropBusy,
  interopContent,
  setNotice,
  encoderInfo,
  selectedExportPreset,
  setSelectedExportPreset,
  exportResolution,
  setExportResolution,
  exportFrameRate,
  setExportFrameRate,
  exportFormat,
  setExportFormat,
  exportFileName,
  setExportFileName,
  exportOutputPath,
  setExportOutputPath,
  chooseExportDestination,
  useHardwareEncoding,
  setUseHardwareEncoding,
  exportMarkedRange,
  setExportMarkedRange,
  invokeMenuCommand,
  activeClip,
  selectedMedia,
  renderPreview,
  rendering,
  renderProgress,
  addExportToQueue,
  exportProject,
}: ExportWorkspaceProps) {
  const outputResolution =
    exportResolution === "source"
      ? { width: project?.settings.width ?? 1920, height: project?.settings.height ?? 1080 }
      : resolutionPresets.find((item) => item.value === exportResolution) ?? {
          width: project?.settings.width ?? 1920,
          height: project?.settings.height ?? 1080,
        };
  const outputFrameRate =
    exportFrameRate === "source"
      ? project?.settings.frame_rate ?? { numerator: 30, denominator: 1 }
      : (() => {
          const [numerator, denominator] = exportFrameRate.split("/").map(Number);
          return { numerator, denominator };
        })();
  const sequenceEnd = Math.max(
    0,
    ...(project?.sequence.tracks ?? []).flatMap((track) =>
      track.clips.map((clip) => clip.timeline_start + clip.duration)
    )
  );
  const markStart = project?.sequence.mark_in ?? 0;
  const markEnd = project?.sequence.mark_out ?? sequenceEnd;
  const rangeDuration = exportMarkedRange ? Math.max(0, markEnd - markStart) : sequenceEnd;
  const hasClips = Boolean(project?.sequence.tracks.some((track) => track.clips.length > 0));
  const outputFolder = exportOutputPath
    ? exportOutputPath.slice(
        0,
        Math.max(exportOutputPath.lastIndexOf("/"), exportOutputPath.lastIndexOf("\\"))
      )
    : "Choose a destination folder";

  return (
    <main className="export-workspace">
      <StudioPanel title="Destinations" className="export-destinations" scrollable>
        <button
          className={`export-destination ${deliverDestination === "render" ? "selected" : ""}`}
          onClick={() => setDeliverDestination("render")}
        >
          <Icon name="export" />
          <span>
            <strong>Media File</strong>
            <small>Render a video file</small>
          </span>
        </button>
        <button
          className={`export-destination ${deliverDestination === "interop" ? "selected" : ""}`}
          onClick={() => setDeliverDestination("interop")}
        >
          <Icon name="deliver" />
          <span>
            <strong>Timeline Interchange</strong>
            <small>FCPXML · Premiere · EDL</small>
          </span>
        </button>
        <div className="batch-queue-container">
          <div className="batch-queue-header">
            <strong>Batch Render Queue ({batchQueue.length})</strong>
            <div className="batch-queue-btn-group">
              <button
                type="button"
                className={`batch-queue-btn ${queuePaused ? "paused" : ""}`}
                onClick={() => void toggleQueuePause()}
                title={queuePaused ? "Resume paused queue" : "Pause render queue"}
              >
                {queuePaused ? "▶ Resume" : "⏸ Pause"}
              </button>
              <button
                type="button"
                className="batch-queue-btn"
                onClick={() => setBatchQueue([])}
                title="Clear queue list"
              >
                Clear
              </button>
            </div>
          </div>
          <div className="batch-queue-list">
            {batchQueue.length === 0 ? (
              <div
                style={{
                  fontSize: "11px",
                  color: "var(--text-muted)",
                  padding: "10px 0",
                  textAlign: "center",
                }}
              >
                Queue is empty. Click "+ Add to Queue" to batch multiple render jobs.
              </div>
            ) : (
              batchQueue.map((item) => (
                <div key={item.id} className="batch-queue-card">
                  <div className="batch-queue-card-top">
                    <strong title={item.name}>{item.name}</strong>
                    <span className={`batch-status-pill ${item.status}`}>{item.status}</span>
                  </div>
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      fontSize: "10px",
                      color: "var(--text-muted)",
                    }}
                  >
                    <span>
                      {item.format} · {item.resolution}
                    </span>
                    <span>{item.status === "completed" ? "Done" : `${Math.round(item.progress * 100)}%`}</span>
                  </div>
                  <div className="batch-queue-progress-bar">
                    <div
                      className="batch-queue-progress-fill"
                      style={{ width: `${Math.round(item.progress * 100)}%` }}
                    />
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </StudioPanel>
      {deliverDestination === "interop" ? (
        <StudioPanel
          title="Timeline Interchange"
          className="export-settings-panel interop-settings-panel"
          scrollable
        >

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
              gap: "16px",
              marginTop: "16px",
            }}
          >
            {/* Apple FCPXML Card */}
            <div
              className="interop-card"
              style={{
                background: "var(--bg-subpanel)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "16px",
                display: "flex",
                flexDirection: "column",
                gap: "10px",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: "14px", color: "var(--accent-primary)" }}>Apple Final Cut Pro</strong>
                <span
                  style={{
                    fontSize: "10px",
                    background: "var(--accent-primary)",
                    color: "#fff",
                    padding: "2px 8px",
                    borderRadius: "10px",
                    fontWeight: 700,
                  }}
                >
                  FCPXML v1.9
                </span>
              </div>
              <p style={{ margin: 0, fontSize: "11.5px", color: "var(--text-sub)", lineHeight: 1.5 }}>
                Exports multi-track clips, audio/video layers, cut coordinates, and asset paths into standard FCPXML.
              </p>
              <div style={{ display: "flex", gap: "8px", marginTop: "auto" }}>
                <button
                  type="button"
                  className="primary"
                  style={{ flex: 1, padding: "7px 10px", fontSize: "11px" }}
                  onClick={() => void handleExportTimelineInterop("fcpxml")}
                  disabled={isInteropBusy}
                >
                  📤 Export FCPXML
                </button>
                <button
                  type="button"
                  className="secondary"
                  style={{ flex: 1, padding: "7px 10px", fontSize: "11px" }}
                  onClick={() => void handleImportTimelineInterop()}
                  disabled={isInteropBusy}
                >
                  📥 Import FCPXML
                </button>
              </div>
            </div>

            {/* Adobe Premiere XML Card */}
            <div
              className="interop-card"
              style={{
                background: "var(--bg-subpanel)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "16px",
                display: "flex",
                flexDirection: "column",
                gap: "10px",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: "14px", color: "var(--accent-violet)" }}>Adobe Premiere Pro</strong>
                <span
                  style={{
                    fontSize: "10px",
                    background: "var(--accent-violet)",
                    color: "#fff",
                    padding: "2px 8px",
                    borderRadius: "10px",
                    fontWeight: 700,
                  }}
                >
                  xmeml v5
                </span>
              </div>
              <p style={{ margin: 0, fontSize: "11.5px", color: "var(--text-sub)", lineHeight: 1.5 }}>
                Standard Premiere Pro sequence exchange format preserving track layouts, in/out ticks, and frame rates.
              </p>
              <div style={{ display: "flex", gap: "8px", marginTop: "auto" }}>
                <button
                  type="button"
                  className="primary"
                  style={{ flex: 1, padding: "7px 10px", fontSize: "11px" }}
                  onClick={() => void handleExportTimelineInterop("premiere_xml")}
                  disabled={isInteropBusy}
                >
                  📤 Export Premiere
                </button>
                <button
                  type="button"
                  className="secondary"
                  style={{ flex: 1, padding: "7px 10px", fontSize: "11px" }}
                  onClick={() => void handleImportTimelineInterop()}
                  disabled={isInteropBusy}
                >
                  📥 Import Premiere
                </button>
              </div>
            </div>

            {/* CMX 3600 EDL Card */}
            <div
              className="interop-card"
              style={{
                background: "var(--bg-subpanel)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "16px",
                display: "flex",
                flexDirection: "column",
                gap: "10px",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: "14px", color: "var(--accent-green)" }}>Broadcast EDL</strong>
                <span
                  style={{
                    fontSize: "10px",
                    background: "var(--accent-green)",
                    color: "#fff",
                    padding: "2px 8px",
                    borderRadius: "10px",
                    fontWeight: 700,
                  }}
                >
                  CMX 3600
                </span>
              </div>
              <p style={{ margin: 0, fontSize: "11.5px", color: "var(--text-sub)", lineHeight: 1.5 }}>
                SMPTE standard non-drop frame edit decision list for conform in DaVinci Resolve color suites.
              </p>
              <div style={{ display: "flex", gap: "8px", marginTop: "auto" }}>
                <button
                  type="button"
                  className="primary"
                  style={{ flex: 1, padding: "7px 10px", fontSize: "11px" }}
                  onClick={() => void handleExportTimelineInterop("edl")}
                  disabled={isInteropBusy}
                >
                  📤 Export EDL
                </button>
                <button
                  type="button"
                  className="secondary"
                  style={{ flex: 1, padding: "7px 10px", fontSize: "11px" }}
                  onClick={() => void handleImportTimelineInterop()}
                  disabled={isInteropBusy}
                >
                  📥 Import EDL
                </button>
              </div>
            </div>
          </div>

          <div style={{ marginTop: "24px", display: "flex", flexDirection: "column", gap: "8px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong>Interchange Payload Viewer</strong>
              <button
                type="button"
                className="secondary"
                style={{ fontSize: "11px", padding: "4px 10px" }}
                disabled={!interopContent}
                onClick={async () => {
                  if (interopContent) {
                    await navigator.clipboard.writeText(interopContent);
                    setNotice("Interchange code copied to clipboard");
                  }
                }}
              >
                📋 Copy Code
              </button>
            </div>
            <textarea
              readOnly
              style={{
                width: "100%",
                background: "var(--bg-subpanel)",
                border: "1px solid var(--border-color)",
                borderRadius: "6px",
                color: "var(--text-main)",
                fontFamily: "monospace",
                fontSize: "11px",
                padding: "10px",
                resize: "vertical",
              }}
              rows={12}
              placeholder="Exported XML or EDL string will be displayed here for instant inspection..."
              value={interopContent}
            />
          </div>
        </StudioPanel>
      ) : (
        <>
          <StudioPanel
            title="Export Settings"
            className="export-settings-panel"
            scrollable
          >
            {/* Live Hardware Acceleration Badge */}
            <div className="export-hw-status-card">
              <div className="hw-status-indicator">
                <span className={`hw-dot ${encoderInfo?.hasHardwareEncoder ? "active" : "fallback"}`} />
                <div className="hw-status-details">
                  <strong>
                    {encoderInfo?.hasHardwareEncoder
                      ? `Hardware Encoder: ${encoderInfo.preferredEncoder.toUpperCase()}`
                      : `CPU Software Encoding Fallback (${encoderInfo?.preferredEncoder || "x264enc"})`}
                  </strong>
                  <small>{encoderInfo?.description || "Probing GStreamer hardware acceleration..."}</small>
                </div>
              </div>
              <div className="hw-codecs-list">
                {(encoderInfo?.availableEncoders ?? ["x264enc"]).map((enc) => (
                  <span
                    key={enc}
                    className={`hw-codec-tag ${enc === encoderInfo?.preferredEncoder ? "preferred" : ""}`}
                    title={`Detected GStreamer codec: ${enc}`}
                  >
                    {enc}
                  </span>
                ))}
              </div>
            </div>

            {/* 1-Click Target Delivery Presets */}
            <div className="export-presets-shelf">
              <div className="export-presets-shelf-header">
                <strong>1-CLICK DELIVERY PRESETS</strong>
                <small>YouTube, TikTok/Reels 9:16, Instagram 1:1, Cinema 4K</small>
              </div>
              <div className="export-presets-cards-grid">
                {exportPresets.map((preset) => {
                  const isSelected = selectedExportPreset === preset.id;
                  return (
                    <button
                      key={preset.id}
                      type="button"
                      className={`export-preset-card ${isSelected ? "selected" : ""}`}
                      onClick={() => {
                        setSelectedExportPreset(preset.id);
                        setExportResolution(preset.resolution);
                        setExportFrameRate(preset.frameRate);
                        setExportFormat(preset.format);
                      }}
                      title={`Configure for ${preset.name}`}
                    >
                      <span className="preset-card-icon">{preset.icon}</span>
                      <div className="preset-card-text">
                        <strong>{preset.name}</strong>
                        <small>{preset.subtitle}</small>
                      </div>
                      <span className="preset-badge">{preset.format.toUpperCase()}</span>
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="export-form-scroll">
              <label className="export-field">
                File name
                <input
                  value={exportFileName}
                  disabled={rendering}
                  onChange={(event) => {
                    setExportFileName(event.target.value);
                    setExportOutputPath("");
                  }}
                  placeholder="Main Edit"
                />
              </label>
              <div className="export-field">
                <span>Location</span>
                <div className="export-location">
                  <small title={outputFolder}>{outputFolder}</small>
                  <button onClick={() => void chooseExportDestination()} disabled={!hasClips || rendering}>
                    Choose…
                  </button>
                </div>
              </div>
              <label className="export-field">
                Preset
                <select
                  disabled={rendering}
                  value={exportResolution}
                  onChange={(event) => {
                    setExportResolution(event.target.value);
                    setSelectedExportPreset("custom");
                  }}
                >
                  <option value="source">
                    Match sequence · {project?.settings.width ?? 1920} × {project?.settings.height ?? 1080}
                  </option>
                  {resolutionPresets
                    .filter((item) => item.value !== "custom")
                    .map((item) => (
                      <option value={item.value} key={item.value}>
                        {item.label}
                      </option>
                    ))}
                </select>
              </label>
              <label className="export-field">
                Format
                <select
                  disabled={rendering}
                  value={exportFormat}
                  onChange={(event) => {
                    setExportFormat(event.target.value as "mp4" | "webm");
                    setExportOutputPath("");
                  }}
                >
                  <option value="mp4">H.264 · MP4</option>
                  <option value="webm">VP8 · WebM</option>
                </select>
              </label>
              <details className="export-settings-group" open>
                <summary>Video</summary>
                <div className="export-fields-grid">
                  <label className="export-field">
                    Frame size
                    <select
                      disabled={rendering}
                      value={exportResolution}
                      onChange={(event) => {
                        setExportResolution(event.target.value);
                        setSelectedExportPreset("custom");
                      }}
                    >
                      <option value="source">Match sequence</option>
                      {resolutionPresets
                        .filter((item) => item.value !== "custom")
                        .map((item) => (
                          <option value={item.value} key={item.value}>
                            {item.width} × {item.height}
                          </option>
                        ))}
                    </select>
                  </label>
                  <label className="export-field">
                    Frame rate
                    <select
                      disabled={rendering}
                      value={exportFrameRate}
                      onChange={(event) => {
                        setExportFrameRate(event.target.value);
                        setSelectedExportPreset("custom");
                      }}
                    >
                      {exportFrameRateOptions.map((item) => (
                        <option value={item.value} key={item.value}>
                          {item.value === "source"
                            ? `${item.label} · ${project?.settings.frame_rate.numerator ?? 30}/${
                                project?.settings.frame_rate.denominator ?? 1
                              } fps`
                            : item.label}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <label className={`export-check ${exportFormat !== "mp4" ? "disabled" : ""}`}>
                  <input
                    type="checkbox"
                    checked={useHardwareEncoding}
                    disabled={rendering || exportFormat !== "mp4"}
                    onChange={(event) => setUseHardwareEncoding(event.target.checked)}
                  />
                  <span>
                    <strong>Use hardware encoder when available</strong>
                    <small>
                      {exportFormat === "mp4"
                        ? "Try your GPU first; software encoding is used if it is unavailable."
                        : "Hardware H.264 applies to MP4 exports."}
                    </small>
                  </span>
                </label>
              </details>
              <details className="export-settings-group" open>
                <summary>Audio</summary>
                <div className="export-audio-summary">
                  <span>Audio format</span>
                  <strong>{exportFormat === "mp4" ? "AAC" : "Vorbis"}</strong>
                  <span>Sample rate</span>
                  <strong>{project?.settings.audio_sample_rate ?? 48_000} Hz · source mix</strong>
                </div>
              </details>
              <details className="export-settings-group">
                <summary>More settings</summary>
                <label className="range-export export-check">
                  <input
                    type="checkbox"
                    checked={exportMarkedRange}
                    disabled={project?.sequence.mark_in == null || project?.sequence.mark_out == null || rendering}
                    onChange={(event) => setExportMarkedRange(event.target.checked)}
                  />
                  <span>
                    <strong>Export marked In/Out range</strong>
                    <small>Use the sequence markers instead of the full timeline.</small>
                  </span>
                </label>
                <div className="export-mark-row">
                  <span>
                    In <b>{project?.sequence.mark_in == null ? "Not set" : formatTime(project.sequence.mark_in)}</b>
                  </span>
                  <span>
                    Out <b>{project?.sequence.mark_out == null ? "Not set" : formatTime(project.sequence.mark_out)}</b>
                  </span>
                  <button
                    disabled={project?.sequence.mark_in == null && project?.sequence.mark_out == null}
                    onClick={() => void invokeMenuCommand("timeline.clear_marks")}
                  >
                    Clear marks
                  </button>
                </div>
              </details>
            </div>
          </StudioPanel>
          <StudioPanel
            title={
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", width: "100%" }}>
                <span>PREVIEW &amp; RENDER</span>
                <span style={{ fontSize: "10px", padding: "1px 6px", borderRadius: "3px", background: "var(--bg-subpanel)", border: "1px solid var(--border-panel)", color: "var(--accent-cyan)", textTransform: "none" }}>{exportFormat === "mp4" ? "H.264" : "VP8"}</span>
              </div>
            }
            className="export-preview-panel"
            scrollable
          >
            <div className="export-preview-stage">{renderPreview(activeClip?.media ?? selectedMedia, true)}</div>
            <div className="export-range-select">
              Range <strong>{exportMarkedRange ? "Marked In / Out" : "Entire sequence"}</strong>
            </div>
            <div className="export-summary">
              <h2>Summary</h2>
              <dl>
                <dt>Output</dt>
                <dd>{exportOutputPath || `${exportFileName || project?.name || "Main Edit"}.${exportFormat}`}</dd>
                <dt>Video</dt>
                <dd>
                  {outputResolution.width} × {outputResolution.height} · {outputFrameRate.numerator}/
                  {outputFrameRate.denominator} fps
                </dd>
                <dt>Engine</dt>
                <dd>
                  {encoderInfo?.hasHardwareEncoder && useHardwareEncoding && exportFormat === "mp4"
                    ? `Hardware Accelerated (${encoderInfo.preferredEncoder})`
                    : "CPU Software (x264enc)"}
                </dd>
                <dt>Container</dt>
                <dd>{exportFormat === "mp4" ? "H.264 (MP4)" : "VP8 (WebM)"}</dd>
                <dt>Audio</dt>
                <dd>
                  {exportFormat === "mp4" ? "AAC" : "Vorbis"} · {project?.settings.audio_sample_rate ?? 48_000} Hz
                </dd>
                <dt>Range</dt>
                <dd>{formatTime(rangeDuration)}</dd>
              </dl>
            </div>
            {renderProgress && (
              <div className="render-progress">
                <progress max={100} value={renderProgress.fractionPercent} />
                <small>
                  {renderProgress.phase} · {renderProgress.fractionPercent}%
                </small>
              </div>
            )}
            <div style={{ display: "flex", gap: "8px", marginTop: "10px" }}>
              <button
                className="secondary"
                style={{ flex: 1, padding: "8px 12px", fontWeight: 700 }}
                disabled={!hasClips || rendering}
                onClick={() => void addExportToQueue()}
              >
                + Add to Queue
              </button>
              <button
                className="primary export-submit"
                style={{ flex: 1 }}
                disabled={!hasClips || rendering}
                onClick={() => void exportProject()}
              >
                {rendering ? "Rendering…" : "Export video"}
              </button>
            </div>
            <small className="export-footer-note">
              Color controls and track gain are included. Animated keyframes are preview-only.
            </small>
          </StudioPanel>
        </>
      )}
    </main>
  );
}
