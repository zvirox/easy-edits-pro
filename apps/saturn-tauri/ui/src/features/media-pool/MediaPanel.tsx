import { InteriorButton } from "../../components/interior/interior-button";
import type { PointerEvent } from "react";
import { Icon, type IconName } from "../../components/Icon";
import { LoadingButton } from "../../components/interior/loading-button";
import { formatTime } from "../../utils/formatters";
import type {
  FilmoraTab,
  MediaAsset,
  OpenverseAsset,
  ProjectDocument,
  TransitionKind,
} from "../../app/projectTypes";

export type { FilmoraTab, OpenverseAsset };

export const filmoraTabs: { id: FilmoraTab; label: string; icon: IconName; tooltip: string }[] = [
  { id: "media", label: "Media", icon: "media", tooltip: "Project Media Bin" },
  { id: "titles", label: "Titles", icon: "title", tooltip: "Titles & Lower-Thirds" },
  { id: "transitions", label: "Transitions", icon: "transition", tooltip: "Visual Transitions" },
  { id: "audio", label: "Audio", icon: "audio", tooltip: "Audio & SFX Library" },
  { id: "stock", label: "Stock", icon: "media", tooltip: "Openverse Stock Media" },
  { id: "ai", label: "AI Tools", icon: "effects", tooltip: "Local AI Assistant" },
];

function getMediaIconName(kind: MediaAsset["kind"]): IconName {
  if (kind === "audio") return "audio";
  if (kind === "title") return "title";
  return "media";
}

export const titlePresets = [
  {
    id: "lower_third",
    name: "Broadcast Lower-Third",
    desc: "Presenter name & role banner",
    preview: "John Doe · Lead Editor",
    style: "lower_third",
    text: "Presenter Name",
    subtitle: "Lead Video Editor",
  },
  {
    id: "minimal_pill",
    name: "Minimal Pill",
    desc: "Modern frosted floating badge",
    preview: "✦ MINIMAL PILL",
    style: "minimal_pill",
    text: "Minimal Pill Headline",
    subtitle: "Subtle contextual note",
  },
  {
    id: "center_bold",
    name: "Cinematic Headline",
    desc: "Bold high-impact centered title",
    preview: "EPISODE 01",
    style: "center_bold",
    text: "CINEMATIC HEADLINE",
    subtitle: "A NEW CHAPTER BEGINS",
  },
  {
    id: "cyber_neon",
    name: "Cyber Neon",
    desc: "Vibrant glowing title effect",
    preview: "⚡ CYBER NEON",
    style: "cyber_neon",
    text: "CYBER NEON 2026",
    subtitle: "NIGHT CITY PROTOCOL",
  },
  {
    id: "headline_subtitle",
    name: "Documentary Card",
    desc: "Headline with location subtext",
    preview: "DOCUMENTARY",
    style: "headline_subtitle",
    text: "Documentary Chapter",
    subtitle: "Location and historical context",
  },
  {
    id: "boxed_banner",
    name: "Framed Banner",
    desc: "Aesthetic boxed caption overlay",
    preview: "[ FRAMED BOX ]",
    style: "boxed_banner",
    text: "Framed Aesthetic Banner",
    subtitle: "Curated sequence notes",
  },
];

export const transitionPresets: { id: TransitionKind; name: string; desc: string; iconSymbol: string }[] = [
  { id: "crossfade", name: "Crossfade", desc: "Smooth blend dissolve between clips", iconSymbol: "⧓" },
  { id: "dip_to_black", name: "Dip to Black", desc: "Cinematic fade through black", iconSymbol: "◼" },
  { id: "dip_to_white", name: "Dip to White", desc: "High-energy flash through white", iconSymbol: "◻" },
  { id: "wipe_left", name: "Wipe Left", desc: "Horizontal wipe moving left", iconSymbol: "◀|" },
  { id: "wipe_right", name: "Wipe Right", desc: "Horizontal wipe moving right", iconSymbol: "|▶" },
  { id: "slide_left", name: "Slide Left", desc: "Push slide moving left", iconSymbol: "⇠" },
  { id: "slide_right", name: "Slide Right", desc: "Push slide moving right", iconSymbol: "⇢" },
];

export interface MediaPanelProps {
  filmoraTab: FilmoraTab;
  setFilmoraTab: (tab: FilmoraTab) => void;
  media: MediaAsset[];
  visibleMedia?: MediaAsset[];
  selectedMediaId: number | null;
  setSelectedMediaId: (id: number) => void;
  busy: boolean;
  checkMissingMedia: () => Promise<void>;
  setCreateBinModalOpen: (open: boolean) => void;
  setNetworkImportModalOpen: (open: boolean) => void;
  importMedia: () => Promise<void>;
  search: string;
  setSearch: (s: string) => void;
  mediaViewMode: "grid" | "list";
  setMediaViewMode: (mode: "grid" | "list") => void;
  activeBinId: number | null;
  setActiveBinId: (id: number | null) => void;
  project: ProjectDocument["project"] | null;
  missingMediaIds: Set<number>;
  setMediaEditPropsId: (id: number | null) => void;
  deleteBin: (id: number) => Promise<void>;
  beginMediaPointer: (event: PointerEvent<HTMLButtonElement>, mediaId: number) => void;
  moveMediaPointer: (event: PointerEvent<HTMLButtonElement>) => void;
  cancelMediaPointer: (event: PointerEvent<HTMLButtonElement>) => void;
  finishMediaPointer?: (event: PointerEvent<HTMLButtonElement>) => void;
  addPresetTitle: (preset: (typeof titlePresets)[0]) => Promise<void>;
  addPresetTransition: (kind: TransitionKind) => Promise<void>;
  playhead: number;
  assetQuery: string;
  setAssetQuery: (q: string) => void;
  assetType: "images" | "audio";
  setAssetType: (t: "images" | "audio") => void;
  searchAssets: () => Promise<void>;
  assetResults: OpenverseAsset[];
  addLibraryAsset: (asset: OpenverseAsset) => Promise<void>;
  setTranscribeModalOpen: (open: boolean) => void;
  handleDetectSilence: () => Promise<void>;
  setSilenceModalOpen: (open: boolean) => void;
  aiModel: string;
  setAiModel: (m: string) => void;
  aiPrompt: string;
  setAiPrompt: (p: string) => void;
  askAi: () => Promise<void>;
  aiReply: string;
}

export function MediaPanel({
  filmoraTab,
  setFilmoraTab,
  media,
  visibleMedia,
  selectedMediaId,
  setSelectedMediaId,
  busy,
  checkMissingMedia,
  setCreateBinModalOpen,
  setNetworkImportModalOpen,
  importMedia,
  search,
  setSearch,
  mediaViewMode,
  setMediaViewMode,
  activeBinId,
  setActiveBinId,
  project,
  missingMediaIds,
  setMediaEditPropsId,
  deleteBin,
  beginMediaPointer,
  moveMediaPointer,
  cancelMediaPointer,
  finishMediaPointer,
  addPresetTitle,
  addPresetTransition,
  playhead,
  assetQuery,
  setAssetQuery,
  assetType,
  setAssetType,
  searchAssets,
  assetResults,
  addLibraryAsset,
  setTranscribeModalOpen,
  handleDetectSilence,
  setSilenceModalOpen,
  aiModel,
  setAiModel,
  aiPrompt,
  setAiPrompt,
  askAi,
  aiReply,
}: MediaPanelProps) {
  const mediaItems = visibleMedia ?? [];
  const allMedia = media ?? [];
  return (
    <aside className="media-panel">
      <div className="filmora-tabs" role="tablist" aria-label="Filmora Asset Categories">
        {filmoraTabs.map((tab) => (
          <InteriorButton
            key={tab.id}
            className={`filmora-tab-btn ${filmoraTab === tab.id ? "active" : ""}`}
            onClick={() => setFilmoraTab(tab.id)}
            data-tooltip={tab.tooltip}
            data-tooltip-pos="bottom"
            role="tab"
            aria-selected={filmoraTab === tab.id}
          >
            <Icon name={tab.icon} />
            <span>{tab.label}</span>
          </InteriorButton>
        ))}
        <InteriorButton
          type="button"
          className="filmora-tab-btn add-tab-btn"
          onClick={() => void importMedia()}
          data-tooltip="Import Media or Add Assets"
          data-tooltip-pos="bottom"
          title="Import Media or Add Assets"
        >
          ＋
        </InteriorButton>
      </div>

      <div className="filmora-panel-content">
        {filmoraTab === "media" && (
          <>
            <div className="panel-heading">
              <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                <strong>Project media</strong>
                <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>({media.length})</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <InteriorButton
                  type="button"
                  className="btn-secondary"
                  style={{ padding: "4px 7px", fontSize: "11px" }}
                  onClick={() => void checkMissingMedia()}
                  disabled={busy}
                  data-tooltip="Scan & relink missing media files"
                  data-tooltip-pos="bottom"
                >
                  🔍 Scan
                </InteriorButton>
                <InteriorButton
                  type="button"
                  className="btn-secondary"
                  style={{ padding: "4px 7px", fontSize: "11px" }}
                  onClick={() => setCreateBinModalOpen(true)}
                  disabled={busy}
                  data-tooltip="Create new media bin / folder"
                  data-tooltip-pos="bottom"
                >
                  ＋ Bin
                </InteriorButton>
                <InteriorButton
                  type="button"
                  className="btn-secondary"
                  style={{ padding: "4px 8px", fontSize: "11px" }}
                  onClick={() => setNetworkImportModalOpen(true)}
                  disabled={busy}
                  data-tooltip="Import from Network / Cloud URL (HTTP/HTTPS/S3)"
                  data-tooltip-pos="bottom"
                >
                  🌐 URL
                </InteriorButton>
                <InteriorButton
                  className="primary"
                  style={{ padding: "4px 8px", fontSize: "11px" }}
                  onClick={() => void importMedia()}
                  disabled={busy}
                  data-tooltip="Import Media Files (Ctrl+I)"
                  data-tooltip-pos="bottom"
                >
                  Import
                </InteriorButton>
              </div>
            </div>

            <div className="media-controls-row">
              <input
                className="search"
                value={search}
                placeholder="Search media, tags…"
                onChange={(event) => setSearch(event.target.value)}
              />
              <div className="media-view-toggle">
                <InteriorButton
                  type="button"
                  className={`view-mode-btn ${mediaViewMode === "grid" ? "active" : ""}`}
                  onClick={() => setMediaViewMode("grid")}
                  title="Thumbnail Grid View"
                >
                  ▦
                </InteriorButton>
                <InteriorButton
                  type="button"
                  className={`view-mode-btn ${mediaViewMode === "list" ? "active" : ""}`}
                  onClick={() => setMediaViewMode("list")}
                  title="Detailed List Table View"
                >
                  ☰
                </InteriorButton>
              </div>
            </div>

            <div className="media-bins-strip scrollbar-thin">
              <InteriorButton
                type="button"
                className={`media-bin-pill ${activeBinId === null ? "active" : ""}`}
                onClick={() => setActiveBinId(null)}
              >
                📁 All ({allMedia.length})
              </InteriorButton>
              {(project?.bins ?? []).map((bin) => {
                const binCount = allMedia.filter((m) => m.bin_id === bin.id).length;
                return (
                  <InteriorButton
                    key={bin.id}
                    type="button"
                    className={`media-bin-pill ${activeBinId === bin.id ? "active" : ""}`}
                    onClick={() => setActiveBinId(bin.id)}
                    style={{ borderLeftColor: bin.color || "var(--accent-primary)" }}
                  >
                    <span>{bin.name}</span>
                    <span className="bin-count">({binCount})</span>
                    <span
                      className="bin-close-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        void deleteBin(bin.id);
                      }}
                      title="Delete Bin"
                    >
                      ×
                    </span>
                  </InteriorButton>
                );
              })}
            </div>

            {mediaViewMode === "grid" ? (
              <div className="media-list">
                {mediaItems.map((item) => {
                  const isMissing = missingMediaIds.has(item.id) || item.is_missing;
                  const isSelected = selectedMediaId === item.id;
                  const binName = project?.bins?.find((b) => b.id === item.bin_id)?.name;
                  return (
                    <div className="media-row" key={item.id} style={{ position: "relative" }}>
                      <InteriorButton
                        className={`media-item ${isSelected ? "selected" : ""} ${isMissing ? "is-missing" : ""}`}
                        data-media-id={item.id}
                        onClick={() => setSelectedMediaId(item.id)}
                        onPointerDown={(event) => beginMediaPointer(event, item.id)}
                        onPointerMove={moveMediaPointer}
                        onPointerUp={(event) => (finishMediaPointer ? void finishMediaPointer(event) : cancelMediaPointer(event))}
                        onPointerCancel={cancelMediaPointer}
                        data-tooltip={
                          isMissing
                            ? `MISSING MEDIA: ${item.path} (Click to Relink)`
                            : `${item.kind.toUpperCase()} · ${item.width && item.height ? `${item.width}×${item.height} · ` : ""}${item.duration ? formatTime(item.duration) : "Still"} · Click & drag to timeline`
                        }
                        data-tooltip-pos="right"
                        style={item.color_tag ? { borderLeft: `3px solid ${item.color_tag}` } : undefined}
                      >
                        <Icon name={getMediaIconName(item.kind)} />
                        <span>
                          <strong>{item.name}</strong>
                          <small>
                            {isMissing ? (
                              <span style={{ color: "#ef4444", fontWeight: 700 }}>⚠ Offline File</span>
                            ) : (
                              <>
                                {item.width && item.height ? `${item.width}×${item.height} · ` : ""}
                                {item.duration ? formatTime(item.duration) : "Still"}
                                {binName ? ` · [${binName}]` : ""}
                              </>
                            )}
                          </small>
                        </span>
                      </InteriorButton>
                      <InteriorButton
                        type="button"
                        className="media-edit-meta-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          setMediaEditPropsId(item.id);
                        }}
                        title="Edit metadata, color tag & comments"
                      >
                        ✎
                      </InteriorButton>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="media-table-wrapper scrollbar-thin">
                <table className="media-details-table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Type</th>
                      <th>Duration</th>
                      <th>Resolution</th>
                      <th>Bin</th>
                      <th>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {mediaItems.map((item) => {
                      const isMissing = missingMediaIds.has(item.id) || item.is_missing;
                      const isSelected = selectedMediaId === item.id;
                      const binName = project?.bins?.find((b) => b.id === item.bin_id)?.name;
                      return (
                        <tr
                          key={item.id}
                          className={`media-table-row ${isSelected ? "selected" : ""}`}
                          data-media-id={item.id}
                          draggable={true}
                          onDragStart={(e) => {
                            e.dataTransfer.setData("application/json", JSON.stringify({ media_id: item.id }));
                            e.dataTransfer.effectAllowed = "copy";
                          }}
                          onClick={() => setSelectedMediaId(item.id)}
                          style={item.color_tag ? { borderLeft: `3px solid ${item.color_tag}` } : undefined}
                        >
                          <td className="cell-name">
                            <span className="table-icon">
                              <Icon name={getMediaIconName(item.kind)} />
                            </span>
                            <span>{item.name}</span>
                            {isMissing && <span className="missing-badge">Offline</span>}
                          </td>
                          <td className="cell-kind">{item.kind}</td>
                          <td className="cell-dur">{item.duration ? formatTime(item.duration) : "—"}</td>
                          <td className="cell-res">
                            {item.width && item.height ? `${item.width}×${item.height}` : "—"}
                          </td>
                          <td className="cell-bin">{binName || "—"}</td>
                          <td className="cell-actions">
                            <InteriorButton
                              type="button"
                              className="btn-tiny"
                              onClick={(e) => {
                                e.stopPropagation();
                                setMediaEditPropsId(item.id);
                              }}
                              title="Edit Tags"
                            >
                              ✎
                            </InteriorButton>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}

            {mediaItems.length === 0 && (
              <div className="empty-media">
                <span className="media-icon">
                  <Icon name="media" />
                </span>
                <strong>
                  {search ? "No matching media" : activeBinId ? "No media in this bin" : "No media imported"}
                </strong>
                <small>
                  {search
                    ? "Try another search query or tag."
                    : activeBinId
                    ? "Move clips into this bin or import files."
                    : "Import video, audio, or images to begin editing."}
                </small>
                <InteriorButton onClick={() => void importMedia()} disabled={busy}>
                  Browse files…
                </InteriorButton>
              </div>
            )}
          </>
        )}

        {filmoraTab === "titles" && (
          <>
            <div className="panel-heading">
              <strong>Title Presets</strong>
              <small>1-Click Add</small>
            </div>
            <div className="filmora-preset-grid">
              {titlePresets.map((preset) => (
                <InteriorButton
                  key={preset.id}
                  className="filmora-preset-card"
                  onClick={() => void addPresetTitle(preset)}
                  data-tooltip={`Add ${preset.name} at playhead (${formatTime(playhead)})`}
                  data-tooltip-pos="right"
                >
                  <div className="filmora-preset-preview">
                    <span>{preset.preview}</span>
                  </div>
                  <span className="filmora-preset-name">{preset.name}</span>
                  <span className="filmora-preset-desc">{preset.desc}</span>
                </InteriorButton>
              ))}
            </div>
          </>
        )}

        {filmoraTab === "transitions" && (
          <>
            <div className="panel-heading">
              <strong>Transitions</strong>
              <small>1-Click Apply</small>
            </div>
            <div className="filmora-preset-grid">
              {transitionPresets.map((preset) => (
                <InteriorButton
                  key={preset.id}
                  className="filmora-preset-card"
                  onClick={() => void addPresetTransition(preset.id)}
                  data-tooltip={`Apply ${preset.name} at cut/playhead`}
                  data-tooltip-pos="right"
                >
                  <div className="filmora-preset-preview">
                    <span style={{ fontSize: "16px" }}>{preset.iconSymbol}</span>
                  </div>
                  <span className="filmora-preset-name">{preset.name}</span>
                  <span className="filmora-preset-desc">{preset.desc}</span>
                </InteriorButton>
              ))}
            </div>
          </>
        )}

        {filmoraTab === "audio" && (
          <>
            <div className="panel-heading">
              <strong>Audio Clips</strong>
            </div>
            <div className="media-list">
              {media
                .filter((item) => item.kind === "audio")
                .map((item) => (
                  <div className="media-row" key={item.id}>
                    <InteriorButton
                      className={`media-item ${selectedMediaId === item.id ? "selected" : ""}`}
                      data-media-id={item.id}
                      onClick={() => setSelectedMediaId(item.id)}
                      data-tooltip={`Audio track · ${item.duration ? formatTime(item.duration) : "Audio"}`}
                      data-tooltip-pos="right"
                    >
                      <Icon name="audio" />
                      <span>
                        <strong>{item.name}</strong>
                        <small>{item.duration ? formatTime(item.duration) : "Audio clip"}</small>
                      </span>
                    </InteriorButton>
                  </div>
                ))}
              {media.filter((item) => item.kind === "audio").length === 0 && (
                <div className="empty-media">
                  <span className="media-icon">
                    <Icon name="audio" />
                  </span>
                  <strong>No audio imported</strong>
                  <small>Import background music, voiceover, or sound effects.</small>
                </div>
              )}
            </div>
          </>
        )}

        {filmoraTab === "stock" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <div className="panel-heading">
              <strong>Stock Media</strong>
              <small>Openverse</small>
            </div>
            <input
              className="search"
              value={assetQuery}
              maxLength={200}
              placeholder="Search photos, audio…"
              onChange={(event) => setAssetQuery(event.target.value)}
            />
            <div style={{ display: "flex", gap: "6px", margin: "4px 9px" }}>
              <select
                style={{
                  flex: 1,
                  padding: "5px",
                  background: "var(--bg-input)",
                  color: "var(--text-main)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "4px",
                }}
                value={assetType}
                onChange={(event) => setAssetType(event.target.value as typeof assetType)}
              >
                <option value="images">Images</option>
                <option value="audio">Audio</option>
              </select>
              <LoadingButton
                className="primary"
                disabled={busy || !assetQuery.trim()}
                onAction={searchAssets}
                pendingLabel="…"
                successLabel="Done"
                errorLabel="Failed"
              >
                Search
              </LoadingButton>
            </div>
            <div className="asset-results" style={{ padding: "0 9px" }}>
              {assetResults.map((asset) => (
                <article className="asset-result" key={`${asset.id}-${asset.mediaUrl}`}>
                  {asset.thumbnailUrl && <img src={asset.thumbnailUrl} alt="" loading="lazy" />}
                  <strong>{asset.title}</strong>
                  <small>
                    {asset.creator ? `By ${asset.creator}` : "Openverse"} · {asset.license ?? "FOSS"}
                  </small>
                  <div className="asset-actions">
                    <InteriorButton disabled={busy} onClick={() => void addLibraryAsset(asset)}>
                      Add to Bin
                    </InteriorButton>
                  </div>
                </article>
              ))}
            </div>
          </div>
        )}

        {filmoraTab === "ai" && (
          <div style={{ padding: "4px 9px", display: "flex", flexDirection: "column", gap: "10px" }}>
            {/* Offline Whisper AI & Ripple Deletion Card */}
            <div
              className="ai-speech-intelligence-card"
              style={{
                background: "rgba(159, 18, 73, 0.05)",
                border: "1px solid rgba(159, 18, 73, 0.22)",
                borderRadius: "6px",
                padding: "10px",
                display: "flex",
                flexDirection: "column",
                gap: "8px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <strong style={{ fontSize: "12px", color: "var(--accent-primary)" }}>🎙️ Speech Intelligence</strong>
                <span
                  style={{
                    fontSize: "9px",
                    background: "#9f1249",
                    color: "#fff",
                    padding: "2px 6px",
                    borderRadius: "8px",
                    fontWeight: "bold",
                  }}
                >
                  Offline Whisper
                </span>
              </div>
              <p style={{ margin: 0, fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.4 }}>
                Automated speech-to-subtitles generation and atomic ripple removal of silence gaps &amp; filler words.
              </p>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px" }}>
                <InteriorButton
                  type="button"
                  className="primary"
                  style={{ fontSize: "11px", padding: "6px 8px", whiteSpace: "nowrap" }}
                  onClick={() => setTranscribeModalOpen(true)}
                >
                  Transcribe Speech
                </InteriorButton>
                <InteriorButton
                  type="button"
                  className="btn-secondary"
                  style={{ fontSize: "11px", padding: "6px 8px", whiteSpace: "nowrap" }}
                  onClick={() => {
                    void handleDetectSilence();
                    setSilenceModalOpen(true);
                  }}
                >
                  Clean Silence/Fillers
                </InteriorButton>
              </div>
            </div>

            <div className="panel-heading" style={{ padding: 0 }}>
              <strong>Local AI Assistant</strong>
              <small>Ollama</small>
            </div>
            <label style={{ fontSize: "11px", color: "var(--text-muted)" }}>
              Model
              <input
                style={{
                  width: "100%",
                  marginTop: "3px",
                  padding: "5px",
                  background: "var(--bg-input)",
                  color: "var(--text-main)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "4px",
                }}
                value={aiModel}
                maxLength={100}
                onChange={(event) => setAiModel(event.target.value)}
              />
            </label>
            <label style={{ fontSize: "11px", color: "var(--text-muted)" }}>
              Prompt
              <textarea
                rows={4}
                style={{
                  width: "100%",
                  marginTop: "3px",
                  padding: "5px",
                  background: "var(--bg-input)",
                  color: "var(--text-main)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "4px",
                  resize: "vertical",
                }}
                value={aiPrompt}
                placeholder="Ask for editing ideas, shot pacing, or script notes…"
                onChange={(event) => setAiPrompt(event.target.value)}
              />
            </label>
            <InteriorButton
              className="primary"
              disabled={busy || !aiPrompt.trim() || !aiModel.trim()}
              onClick={() => void askAi()}
            >
              Generate Suggestions
            </InteriorButton>
            {aiReply && <pre className="ai-reply">{aiReply}</pre>}
          </div>
        )}
      </div>
    </aside>
  );
}
