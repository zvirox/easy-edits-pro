import { invoke } from "@tauri-apps/api/core";

export interface InteropModalProps {
  isOpen: boolean;
  onClose: () => void;
  interopMode: "export" | "import";
  interopFormat: "fcpxml" | "premiere_xml" | "edl" | "otio";
  setInteropFormat: (fmt: "fcpxml" | "premiere_xml" | "edl" | "otio") => void;
  interopContent: string;
  setInteropContent: (c: string) => void;
  interopProjectName: string;
  setInteropProjectName: (name: string) => void;
  isInteropBusy: boolean;
  setIsInteropBusy: (b: boolean) => void;
  handleExportTimelineInterop: (format: string) => Promise<void>;
  handleImportTimelineInterop: () => Promise<void>;
  refreshProject: () => Promise<void>;
  setNotice: (msg: string) => void;
}

export function InteropModal({
  isOpen,
  onClose,
  interopMode,
  interopFormat,
  setInteropFormat,
  interopContent,
  setInteropContent,
  interopProjectName,
  setInteropProjectName,
  isInteropBusy,
  setIsInteropBusy,
  handleExportTimelineInterop,
  handleImportTimelineInterop,
  refreshProject,
  setNotice,
}: InteropModalProps) {
  if (!isOpen) return null;

  return (
    <div className="dialog-backdrop" role="presentation">
      <div className="modal-dialog-card interop-modal" style={{ maxWidth: "600px" }}>
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h2 style={{ margin: 0, fontSize: "16px" }}>
            {interopMode === "export" ? "📤 Export Timeline Interchange" : "📥 Import Timeline Interchange"}
          </h2>
          <button type="button" className="close-btn" onClick={onClose}>✕</button>
        </div>
        <div style={{ display: "flex", gap: "6px", marginBottom: "12px" }}>
          {(["fcpxml", "premiere_xml", "edl", "otio"] as const).map((fmt) => (
            <button
              key={fmt}
              type="button"
              className={`interop-tab-btn ${interopFormat === fmt ? "active" : ""}`}
              onClick={() => {
                setInteropFormat(fmt);
                if (interopMode === "export") void handleExportTimelineInterop(fmt);
              }}
            >
              {fmt === "fcpxml" ? "Apple FCPXML v1.9" : fmt === "premiere_xml" ? "Adobe Premiere XML" : fmt === "edl" ? "CMX 3600 EDL" : "OpenTimelineIO (.otio)"}
            </button>
          ))}
        </div>

        {interopMode === "export" ? (
          <div>
            <p style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "8px" }}>
              Formatted XML/EDL code generated with rational tick timebase:
            </p>
            <textarea
              readOnly
              rows={10}
              style={{
                width: "100%",
                background: "#090d14",
                border: "1px solid #334155",
                borderRadius: "6px",
                color: "#e2e8f0",
                fontFamily: "monospace",
                fontSize: "11px",
                padding: "8px",
                resize: "vertical",
              }}
              value={interopContent}
            />
            <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px", marginTop: "12px" }}>
              <button type="button" onClick={onClose}>Close</button>
              <button
                type="button"
                className="btn-secondary"
                onClick={async () => {
                  await navigator.clipboard.writeText(interopContent);
                  setNotice("Copied to clipboard");
                }}
              >
                📋 Copy Code
              </button>
              <button
                type="button"
                className="primary"
                onClick={() => void handleExportTimelineInterop(interopFormat)}
              >
                💾 Save As File…
              </button>
            </div>
          </div>
        ) : (
          <div>
            <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "8px" }}>
              Project Name
              <input
                value={interopProjectName}
                onChange={(e) => setInteropProjectName(e.target.value)}
                style={{ padding: "6px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
              />
            </label>
            <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "12px" }}>
              Paste XML or EDL Content Below
              <textarea
                rows={8}
                placeholder="Paste FCPXML, Premiere XML, or CMX 3600 EDL text here..."
                value={interopContent}
                onChange={(e) => setInteropContent(e.target.value)}
                style={{
                  width: "100%",
                  padding: "6px",
                  background: "#090d14",
                  border: "1px solid #334155",
                  borderRadius: "6px",
                  color: "#e2e8f0",
                  fontFamily: "monospace",
                  fontSize: "11px",
                  resize: "vertical",
                }}
              />
            </label>
            <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
              <button type="button" onClick={onClose}>Cancel</button>
              <button
                type="button"
                className="btn-secondary"
                onClick={() => void handleImportTimelineInterop()}
              >
                📂 Browse File…
              </button>
              <button
                type="button"
                className="primary"
                disabled={!interopContent.trim() || isInteropBusy}
                onClick={async () => {
                  setIsInteropBusy(true);
                  try {
                    await invoke("import_timeline_interop", {
                      format: interopFormat,
                      content: interopContent,
                      path: null,
                      projectName: interopProjectName,
                    });
                    await refreshProject();
                    onClose();
                    setNotice(`Imported ${interopFormat.toUpperCase()} timeline successfully`);
                  } catch (e) {
                    setNotice(`Import error: ${String(e)}`);
                  } finally {
                    setIsInteropBusy(false);
                  }
                }}
              >
                Import Timeline
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
