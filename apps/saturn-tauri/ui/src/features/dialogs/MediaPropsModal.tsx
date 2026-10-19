import { InteriorButton } from "../../components/interior/interior-button";
import type { MediaAsset, ProjectDocument } from "../../app/projectTypes";

export interface MediaPropsModalProps {
  mediaEditPropsId: number | null;
  onClose: () => void;
  mediaEditPropsName: string;
  setMediaEditPropsName: (name: string) => void;
  mediaEditPropsColor: string;
  setMediaEditPropsColor: (color: string) => void;
  mediaEditPropsTags: string;
  setMediaEditPropsTags: (tags: string) => void;
  media: MediaAsset[];
  project: ProjectDocument["project"] | null;
  moveMediaToBin: (id: number, binId: number | null) => Promise<void>;
  updateMediaProperties: (
    id: number,
    name: string,
    color?: string,
    tags?: string[]
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
  media,
  project,
  moveMediaToBin,
  updateMediaProperties,
}: MediaPropsModalProps) {
  if (mediaEditPropsId === null) return null;

  return (
    <div className="dialog-backdrop" role="presentation">
      <form
        className="modal-dialog-card"
        style={{ maxWidth: "460px" }}
        onSubmit={(e) => {
          e.preventDefault();
          const tags = mediaEditPropsTags.split(",").map((t) => t.trim()).filter(Boolean);
          void updateMediaProperties(mediaEditPropsId, mediaEditPropsName, mediaEditPropsColor || undefined, tags);
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
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "16px" }}>
          Organize in Media Bin
          <select
            value={media.find((m) => m.id === mediaEditPropsId)?.bin_id ?? ""}
            onChange={(e) => {
              const val = e.target.value ? Number(e.target.value) : null;
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
        <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
          <InteriorButton type="button" onClick={onClose}>Cancel</InteriorButton>
          <InteriorButton className="primary" type="submit">Save Properties</InteriorButton>
        </div>
      </form>
    </div>
  );
}
