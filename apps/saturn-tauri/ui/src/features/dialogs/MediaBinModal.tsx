import { InteriorButton } from "../../components/interior/interior-button";
import { GlassDialog } from "../../motion/GlassDialog";
export interface MediaBinModalProps {
  isOpen: boolean;
  onClose: () => void;
  newBinName: string;
  setNewBinName: (name: string) => void;
  newBinColor: string;
  setNewBinColor: (color: string) => void;
  createMediaBin: (name: string, color: string) => Promise<void>;
}

export function MediaBinModal({
  isOpen,
  onClose,
  newBinName,
  setNewBinName,
  newBinColor,
  setNewBinColor,
  createMediaBin,
}: MediaBinModalProps) {
  return (
    <GlassDialog
      as="form"
      open={isOpen}
      cardClassName="modal-dialog-card glass"
      cardProps={{
        style: { maxWidth: "420px" },
        onSubmit: (e) => {
          e.preventDefault();
          void createMediaBin(newBinName, newBinColor);
        },
      }}
    >
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h2 style={{ margin: 0, fontSize: "16px" }}>📁 Create New Media Bin</h2>
          <InteriorButton type="button" className="close-btn" onClick={onClose}>✕</InteriorButton>
        </div>
        <p className="modal-desc" style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "14px" }}>
          Organize project clips, B-roll, music, and titles into color-coded media bins.
        </p>
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "12px" }}>
          Bin Name
          <input
            autoFocus
            type="text"
            placeholder="e.g. B-Roll, Interview Audio, VFX"
            value={newBinName}
            onChange={(e) => setNewBinName(e.target.value)}
            style={{ padding: "8px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
          />
        </label>
        <div style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "16px" }}>
          <span>Color Label</span>
          <div style={{ display: "flex", gap: "8px", alignItems: "center", marginTop: "4px" }}>
            {["#be185d", "#10b981", "#f59e0b", "#ef4444", "#6b21a8", "#ec4899", "#7d6377"].map((c) => (
              <InteriorButton
                key={c}
                type="button"
                onClick={() => setNewBinColor(c)}
                style={{
                  width: "24px",
                  height: "24px",
                  borderRadius: "50%",
                  background: c,
                  border: newBinColor === c ? "2px solid #ffffff" : "2px solid transparent",
                  cursor: "pointer",
                  outline: "none",
                }}
              />
            ))}
          </div>
        </div>
        <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
          <InteriorButton type="button" onClick={onClose}>Cancel</InteriorButton>
          <InteriorButton className="primary" type="submit" disabled={!newBinName.trim()}>Create Bin</InteriorButton>
        </div>
    </GlassDialog>
  );
}
