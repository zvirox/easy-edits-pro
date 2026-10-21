import { InteriorButton } from "../../components/interior/interior-button";
import { GlassDialog } from "../../motion/GlassDialog";
export interface CompoundClipModalProps {
  isOpen: boolean;
  onClose: () => void;
  compoundClipNameDraft: string;
  setCompoundClipNameDraft: (name: string) => void;
  createCompoundClip: (name: string) => Promise<void>;
}

export function CompoundClipModal({
  isOpen,
  onClose,
  compoundClipNameDraft,
  setCompoundClipNameDraft,
  createCompoundClip,
}: CompoundClipModalProps) {
  return (
    <GlassDialog
      as="form"
      open={isOpen}
      cardClassName="modal-dialog-card glass compound-modal"
      cardProps={{
        style: { maxWidth: "440px" },
        onSubmit: (e) => {
          e.preventDefault();
          onClose();
          void createCompoundClip(compoundClipNameDraft);
        },
      }}
    >
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h2 style={{ margin: 0, fontSize: "16px" }}>⧉ New Compound Clip</h2>
          <InteriorButton type="button" className="close-btn" onClick={onClose}>✕</InteriorButton>
        </div>
        <p className="modal-desc" style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "14px" }}>
          Combine selected clips across tracks into a single nested compound sequence. You can double-click it at any time to open and edit inside.
        </p>
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "6px", marginBottom: "16px" }}>
          Compound Clip Name
          <input
            autoFocus
            type="text"
            value={compoundClipNameDraft}
            onChange={(e) => setCompoundClipNameDraft(e.target.value)}
            placeholder="e.g. Intro Sequence, B-Roll Montage"
            style={{ padding: "8px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
          />
        </label>
        <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
          <InteriorButton type="button" onClick={onClose}>Cancel</InteriorButton>
          <InteriorButton className="primary" type="submit" disabled={!compoundClipNameDraft.trim()}>Create Compound Clip</InteriorButton>
        </div>
    </GlassDialog>
  );
}
