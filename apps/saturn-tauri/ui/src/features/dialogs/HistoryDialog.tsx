import { useEffect, useState } from "react";
import { GlassDialog } from "../../motion/GlassDialog";
import { InteriorButton } from "../../components/interior/interior-button";

export interface HistoryDialogProps {
  isOpen: boolean;
  onClose: () => void;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
}

interface HistoryEntry {
  label: string;
}

/**
 * Read-only undo/redo entry list from `history.list`. The engine labels each
 * snapshot with the command that produced it; entries render newest first.
 */
export function HistoryDialog({ isOpen, onClose, runCommand }: HistoryDialogProps) {
  const [undo, setUndo] = useState<HistoryEntry[]>([]);
  const [redo, setRedo] = useState<HistoryEntry[]>([]);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!isOpen) return;
    let active = true;
    setError("");
    runCommand<{ undo: HistoryEntry[]; redo: HistoryEntry[] }>("history.list")
      .then((history) => {
        if (!active) return;
        setUndo(history.undo ?? []);
        setRedo(history.redo ?? []);
      })
      .catch((cause) => {
        if (active) setError(String(cause));
      });
    return () => {
      active = false;
    };
  }, [isOpen, runCommand]);

  return (
    <GlassDialog
      open={isOpen}
      cardClassName="modal-dialog-card glass history-card"
      onBackdropClick={onClose}
      cardProps={{
        style: { maxWidth: "460px", width: "90vw" },
        onClick: (event) => event.stopPropagation(),
      }}
    >
      <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
        <h2 style={{ margin: 0, fontSize: "16px" }}>Undo History</h2>
        <InteriorButton type="button" className="close-btn" onClick={onClose}>
          ×
        </InteriorButton>
      </div>
      {error ? (
        <p className="modal-desc" role="alert">
          Could not read the undo history: {error}
        </p>
      ) : (
        <div className="history-view" data-testid="history-view">
          <h3 className="history-view-title">Undoable ({undo.length})</h3>
          <ul className="history-list" aria-label="Undoable history entries">
            {undo.length === 0 ? (
              <li className="history-empty">Nothing to undo</li>
            ) : (
              undo.map((entry, index) => (
                <li key={`${entry.label}-${index}`} className={index === 0 ? "history-row is-newest" : "history-row"}>
                  {entry.label}
                </li>
              ))
            )}
          </ul>
          <h3 className="history-view-title">Redoable ({redo.length})</h3>
          <ul className="history-list" aria-label="Redoable history entries">
            {redo.length === 0 ? (
              <li className="history-empty">Nothing to redo</li>
            ) : (
              redo.map((entry, index) => (
                <li key={`${entry.label}-${index}`} className="history-row">
                  {entry.label}
                </li>
              ))
            )}
          </ul>
        </div>
      )}
      <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end", marginTop: "12px" }}>
        <InteriorButton size="sm" variant="secondary" onClick={onClose}>
          Close
        </InteriorButton>
      </div>
    </GlassDialog>
  );
}
