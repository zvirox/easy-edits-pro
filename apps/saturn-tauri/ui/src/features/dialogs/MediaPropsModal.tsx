import { useEffect, useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { GlassDialog } from "../../motion/GlassDialog";
import type { MediaAsset, ProjectDocument } from "../../app/projectTypes";
import type { ColorInfo, MediaHygieneOps } from "../../app/useMediaHygiene";

export interface MediaPropsModalProps {
  mediaEditPropsId: number | null;
  onClose: () => void;
  mediaEditPropsName: string;
  setMediaEditPropsName: (name: string) => void;
  mediaEditPropsColor: string;
  setMediaEditPropsColor: (color: string) => void;
  mediaEditPropsTags: string;
  setMediaEditPropsTags: (tags: string) => void;
  mediaEditPropsComment: string;
  setMediaEditPropsComment: (comment: string) => void;
  mediaEditPropsScene: string;
  setMediaEditPropsScene: (scene: string) => void;
  mediaEditPropsTake: string;
  setMediaEditPropsTake: (take: string) => void;
  media: MediaAsset[];
  project: ProjectDocument["project"] | null;
  moveMediaToBin: (id: number, binId: number | null) => Promise<void>;
  /** Engine-backed lifecycle ops: colour info, proxies, offline. */
  hygiene: MediaHygieneOps;
  updateMediaProperties: (
    id: number,
    name: string,
    color?: string,
    tags?: string[],
    metadata?: { comment?: string; scene?: string; take?: string }
  ) => Promise<void>;
}

export function MediaPropsModal({
  mediaEditPropsId,
  onClose,
  mediaEditPropsName,
  setMediaEditPropsName,
  mediaEditPropsColor,
  setMediaEditPropsColor,
  mediaEditPropsTags,
  setMediaEditPropsTags,
  mediaEditPropsComment,
  setMediaEditPropsComment,
  mediaEditPropsScene,
  setMediaEditPropsScene,
  mediaEditPropsTake,
  setMediaEditPropsTake,
  media,
  project,
  moveMediaToBin,
  hygiene,
  updateMediaProperties,
}: MediaPropsModalProps) {
  const [colorInfo, setColorInfo] = useState<ColorInfo | null>(null);

  // media.color_info: read the asset's colour metadata when the dialog opens.
  useEffect(() => {
    setColorInfo(null);
    if (mediaEditPropsId === null) return;
    let active = true;
    void hygiene.colorInfo(mediaEditPropsId).then((info) => {
      if (active) setColorInfo(info);
    });
    return () => {
      active = false;
    };
    // Only the asset id drives the fetch; hygiene is stable across renders.
  }, [mediaEditPropsId]);

  const asset = media.find((m) => m.id === mediaEditPropsId) ?? null;
  const proxyPath = asset?.proxy_path ?? null;
  return (
    <GlassDialog
      as="form"
      open={mediaEditPropsId !== null}
      cardClassName="modal-dialog-card glass"
      cardProps={{
        style: { maxWidth: "460px" },
        onSubmit: (e) => {
          e.preventDefault();
          if (mediaEditPropsId === null) return;
          const tags = mediaEditPropsTags.split(",").map((t) => t.trim()).filter(Boolean);
          void updateMediaProperties(
            mediaEditPropsId,
            mediaEditPropsName,
            mediaEditPropsColor || undefined,
            tags,
            {
              comment: mediaEditPropsComment,
              scene: mediaEditPropsScene,
              take: mediaEditPropsTake,
            },
          );
        },
      }}
    >
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h2 style={{ margin: 0, fontSize: "16px" }}>🏷️ Edit Asset Properties</h2>
          <InteriorButton type="button" className="close-btn" onClick={onClose}>✕</InteriorButton>
        </div>
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "12px" }}>
          Asset Name
          <input
            type="text"
            value={mediaEditPropsName}
            onChange={(e) => setMediaEditPropsName(e.target.value)}
            style={{ padding: "8px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
          />
        </label>
        <div style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "12px" }}>
          <span>Color Label</span>
          <div style={{ display: "flex", gap: "8px", alignItems: "center", marginTop: "4px" }}>
            {["", "#be185d", "#10b981", "#f59e0b", "#ef4444", "#6b21a8", "#ec4899", "#7d6377"].map((c) => (
              <InteriorButton
                key={c || "none"}
                type="button"
                onClick={() => setMediaEditPropsColor(c)}
                style={{
                  width: "24px",
                  height: "24px",
                  borderRadius: "50%",
                  background: c || "transparent",
                  border: mediaEditPropsColor === c ? "2px solid #ffffff" : "1px solid #7d6377",
                  cursor: "pointer",
                  outline: "none",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: "10px",
                  color: "#bfa5b8",
                }}
              >
                {!c && "✕"}
              </InteriorButton>
            ))}
          </div>
        </div>
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "12px" }}>
          Tags (Comma-separated)
          <input
            type="text"
            placeholder="e.g. interview, b-roll, drone, 4k"
            value={mediaEditPropsTags}
            onChange={(e) => setMediaEditPropsTags(e.target.value)}
            style={{ padding: "8px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
          />
        </label>
        <div className="media-metadata-grid">
          <label>
            Scene
            <input
              type="text"
              placeholder="12B"
              value={mediaEditPropsScene}
              onChange={(e) => setMediaEditPropsScene(e.target.value)}
            />
          </label>
          <label>
            Take
            <input
              type="text"
              placeholder="3"
              value={mediaEditPropsTake}
              onChange={(e) => setMediaEditPropsTake(e.target.value)}
            />
          </label>
        </div>
        <label className="media-metadata-comment">
          Comment
          <textarea
            rows={3}
            placeholder="Notes that travel with the asset (never rendered)"
            value={mediaEditPropsComment}
            onChange={(e) => setMediaEditPropsComment(e.target.value)}
          />
        </label>
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "16px" }}>
          Organize in Media Bin
          <select
            value={media.find((m) => m.id === mediaEditPropsId)?.bin_id ?? ""}
            onChange={(e) => {
              const val = e.target.value ? Number(e.target.value) : null;
              if (mediaEditPropsId === null) return;
              void moveMediaToBin(mediaEditPropsId, val);
            }}
            style={{ padding: "8px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
          >
            <option value="">📁 Root / All Media (No Bin)</option>
            {(project?.bins ?? []).map((b) => (
              <option key={b.id} value={b.id}>📁 {b.name}</option>
            ))}
          </select>
        </label>
        <div className="media-hygiene-section" style={{ fontSize: "11px", marginBottom: "14px", display: "grid", gap: "6px" }}>
          <span style={{ color: "var(--text-muted)", fontWeight: 600 }}>Colour &amp; Media State</span>
          <div className="media-metadata-grid">
            <span>
              Colour space: {colorInfo?.colorspace ?? "unknown"}
            </span>
            <span>
              Gamma: {colorInfo?.gamma ?? "unknown"}
            </span>
          </div>
          <div style={{ display: "flex", gap: "8px", alignItems: "center", flexWrap: "wrap" }}>
            <span style={{ fontFamily: "ui-monospace, monospace", fontSize: "10px" }}>
              {proxyPath ? `Proxy: ${proxyPath}` : "No proxy attached"}
            </span>
            <InteriorButton type="button" onClick={() => mediaEditPropsId !== null && void hygiene.attachProxy(mediaEditPropsId)}>
              Attach Proxy…
            </InteriorButton>
            {proxyPath ? (
              <InteriorButton type="button" onClick={() => mediaEditPropsId !== null && void hygiene.detachProxy(mediaEditPropsId)}>
                Detach Proxy
              </InteriorButton>
            ) : null}
          </div>
          {asset && !asset.is_missing && asset.kind !== "title" && asset.kind !== "compound_sequence" ? (
            <div>
              <InteriorButton
                type="button"
                onClick={() => {
                  if (mediaEditPropsId !== null) {
                    void hygiene.makeOffline(mediaEditPropsId);
                    onClose();
                  }
                }}
              >
                Mark Offline
              </InteriorButton>
            </div>
          ) : null}
        </div>
        <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
          <InteriorButton type="button" onClick={onClose}>Cancel</InteriorButton>
          <InteriorButton className="primary" type="submit">Save Properties</InteriorButton>
        </div>
    </GlassDialog>
  );
}
