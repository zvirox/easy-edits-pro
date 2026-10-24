import { useCallback, useEffect, useState } from "react";
import { GlassDialog } from "../../motion/GlassDialog";
import { InteriorButton } from "../../components/interior/interior-button";

export interface ProjectJsonDialogProps {
  isOpen: boolean;
  onClose: () => void;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  setNotice: (message: string) => void;
}

/**
 * Read-only view of the full project document as JSON, straight from
 * `project.inspect`. Handy for debugging and for parity work against the
 * engine source of truth.
 */
export function ProjectJsonDialog({ isOpen, onClose, runCommand, setNotice }: ProjectJsonDialogProps) {
  const [json, setJson] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    if (!isOpen) return;
    let active = true;
    setError("");
    setJson("");
    runCommand<unknown>("project.inspect")
      .then((document) => {
        if (active) setJson(JSON.stringify(document, null, 2));
      })
      .catch((cause) => {
        if (active) setError(String(cause));
      });
    return () => {
      active = false;
    };
  }, [isOpen, runCommand]);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(json);
      setNotice("Project JSON copied to the clipboard");
    } catch (cause) {
      setNotice(`Could not copy: ${String(cause)}`);
    }
  }, [json, setNotice]);

  return (
    <GlassDialog
      open={isOpen}
      cardClassName="modal-dialog-card glass project-json-card"
      onBackdropClick={onClose}
      cardProps={{
        style: { maxWidth: "760px", width: "92vw" },
        onClick: (event) => event.stopPropagation(),
      }}
    >
      <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
        <h2 style={{ margin: 0, fontSize: "16px" }}>Project JSON</h2>
        <InteriorButton type="button" className="close-btn" onClick={onClose}>
          ×
        </InteriorButton>
      </div>
      {error ? (
        <p className="modal-desc" role="alert">
          Could not read the project document: {error}
        </p>
      ) : (
        <pre className="project-json-view" data-testid="project-json" tabIndex={0}>
          {json || "Loading…"}
        </pre>
      )}
      <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end", gap: "8px", marginTop: "12px" }}>
        <InteriorButton size="sm" onClick={() => void copy()} disabled={!json}>
          Copy
        </InteriorButton>
        <InteriorButton size="sm" variant="secondary" onClick={onClose}>
          Close
        </InteriorButton>
      </div>
    </GlassDialog>
  );
}
