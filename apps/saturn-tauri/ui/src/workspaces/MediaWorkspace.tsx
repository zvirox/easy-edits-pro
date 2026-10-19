import { InteriorButton } from "../components/interior/interior-button";
import type { SelectedClip, MediaAsset, ProjectDocument, DavinciWorkspace } from "../types";
import { Icon } from "../components/Icon";
import { formatTime } from "../utils/formatters";

export interface MediaWorkspaceProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  visibleMedia: MediaAsset[];
  selectedMediaId: number | null;
  setSelectedMediaId: (id: number | null) => void;
  setSelectedClip: (val: SelectedClip | null) => void;
  setSelectedClips: (val: SelectedClip[]) => void;
  selectDavinciWorkspace: (ws: DavinciWorkspace) => void;
  importMedia: () => Promise<void> | void;
  setNetworkImportModalOpen: (val: boolean) => void;
  search: string;
  setSearch: (val: string) => void;
  busy: boolean;
  missingMediaIds: Set<number>;
}

export function MediaWorkspace({
  project,
  media,
  visibleMedia,
  selectedMediaId,
  setSelectedMediaId,
  setSelectedClip,
  setSelectedClips,
  selectDavinciWorkspace,
  importMedia,
  setNetworkImportModalOpen,
  search,
  setSearch,
  busy,
  missingMediaIds,
}: MediaWorkspaceProps) {
  return (
    <main className="workspace-page import-workspace" style={{ padding: "16px 20px", gap: "14px" }}>
      {/* Compact Studio Header */}
      <header
        className="workspace-page-heading"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          paddingBottom: "8px",
          borderBottom: "1px solid var(--border-panel)",
        }}
      >
        <div style={{ display: "flex", alignItems: "baseline", gap: "10px" }}>
          <strong style={{ fontSize: "14px", fontWeight: 700, letterSpacing: "0.04em", color: "var(--text-primary)" }}>
            MEDIA POOL
          </strong>
          <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
            {media.length} item{media.length === 1 ? "" : "s"} in {project?.name ?? "this project"}
          </span>
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <InteriorButton
            className="primary workspace-heading-action"
            style={{ padding: "6px 12px", fontSize: "11px" }}
            onClick={() => void importMedia()}
            disabled={busy}
          >
            <Icon name="import" /> Browse Local Files…
          </InteriorButton>
          <InteriorButton
            className="btn-secondary"
            style={{ padding: "6px 12px", fontSize: "11px", background: "var(--bg-elevated)", color: "var(--accent-primary)" }}
            onClick={() => setNetworkImportModalOpen(true)}
            disabled={busy}
          >
            🌐 Import from URL / S3
          </InteriorButton>
        </div>
      </header>

      {/* Drop / Import Banner */}
      <section
        className="import-drop-panel"
        style={{
          background: "var(--bg-elevated)",
          border: "1px dashed var(--border-panel)",
          borderRadius: "6px",
          padding: "14px 18px",
          display: "flex",
          alignItems: "center",
          gap: "16px",
        }}
      >
        <div className="import-drop-icon" style={{ background: "var(--bg-input)", borderColor: "var(--border-panel)" }}>
          <Icon name="media" />
        </div>
        <div style={{ flex: 1 }}>
          <h2 style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)", margin: "0 0 3px 0" }}>
            Add media to your project
          </h2>
          <p style={{ fontSize: "11px", color: "var(--text-secondary)", margin: 0 }}>
            Choose video, audio, or still images from your computer or cloud URL. Imported files stay in their original location.
          </p>
        </div>
      </section>

      {/* Library Grid Section */}
      <section className="import-library-section" style={{ flex: 1, minHeight: 0 }}>
        <header style={{ marginBottom: "12px" }}>
          <div>
            <h2 style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>Project Media</h2>
          </div>
          <input
            className="search"
            type="search"
            placeholder="Search project media"
            aria-label="Search project media"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            style={{ background: "var(--bg-input)", color: "var(--text-primary)", borderColor: "var(--border-panel)" }}
          />
        </header>

        {visibleMedia.length ? (
          <div className="import-media-grid" style={{ overflowY: "auto", paddingBottom: "20px" }}>
            {visibleMedia.map((item) => (
              <InteriorButton
                className={`import-media-card ${selectedMediaId === item.id ? "selected" : ""}`}
                key={item.id}
                onClick={() => {
                  setSelectedMediaId(item.id);
                  setSelectedClip(null);
                  setSelectedClips([]);
                  selectDavinciWorkspace("edit");
                }}
                title="Select this media and open the Edit workspace"
                style={{
                  background: "var(--bg-panel)",
                  borderColor: selectedMediaId === item.id ? "var(--accent-primary)" : "var(--border-panel)",
                }}
              >
                <span className="import-media-icon" style={{ background: "var(--bg-elevated)" }}>
                  <Icon name="media" />
                </span>
                <strong style={{ color: "var(--text-primary)" }}>{item.name}</strong>
                <small style={{ color: "var(--text-secondary)" }}>
                  {item.kind.toUpperCase()} {item.width && item.height ? `· ${item.width} × ${item.height}` : ""}
                </small>
                <small style={{ color: "var(--text-muted)" }}>
                  {item.duration
                    ? formatTime(item.duration)
                    : missingMediaIds.has(item.id)
                    ? "Missing · relink required"
                    : "Ready to edit"}
                </small>
              </InteriorButton>
            ))}
          </div>
        ) : (
          <div className="import-empty" style={{ background: "var(--bg-panel)", borderColor: "var(--border-panel)" }}>
            <Icon name="media" />
            <strong style={{ color: "var(--text-primary)" }}>
              {search ? "No matching media" : "Your project is ready for media"}
            </strong>
            <span style={{ color: "var(--text-secondary)" }}>
              {search ? "Try another search." : "Import a clip to begin building your timeline."}
            </span>
            <InteriorButton onClick={() => void importMedia()} disabled={busy}>
              Browse files…
            </InteriorButton>
          </div>
        )}
      </section>
    </main>
  );
}
