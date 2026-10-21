import { InteriorButton } from "./interior/interior-button";
import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface NetworkImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export const NetworkImportModal: React.FC<NetworkImportModalProps> = ({
  isOpen,
  onClose,
  onSuccess,
}) => {
  const [url, setUrl] = useState("");
  const [customName, setCustomName] = useState("");
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progressText, setProgressText] = useState("");

  if (!isOpen) return null;

  const handleImport = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!url.trim()) {
      setError("Please enter a valid HTTP, HTTPS, or Cloud URL.");
      return;
    }

    setDownloading(true);
    setError(null);
    setProgressText("Connecting to network asset and downloading payload...");

    try {
      await invoke("import_network_media", {
        url: url.trim(),
        customName: customName.trim() ? customName.trim() : null,
      });

      setProgressText("Asset probing and waveform peaks completed!");
      setTimeout(() => {
        setDownloading(false);
        setUrl("");
        setCustomName("");
        onSuccess();
        onClose();
      }, 500);
    } catch (err: unknown) {
      setDownloading(false);
      setError(typeof err === "string" ? err : (err as Error).message || "Failed to download network asset");
    }
  };

  const sampleUrls = [
    { label: "Sample Trailer (MP4)", url: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4" },
    { label: "Nature 4K Clip (WebM)", url: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4" },
    { label: "Openverse Cloud Audio", url: "https://upload.wikimedia.org/wikipedia/commons/4/4b/Apollo_11_launch.ogg" },
  ];

  return (
    <div className="modal-backdrop" onClick={onClose} style={{ zIndex: 10000 }}>
      <div
        className="modal-card network-import-modal glass"
        data-elev="4"
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 540,
          borderRadius: 8,
          overflow: "hidden",
        }}
      >
        <header
          style={{
            padding: "16px 20px",
            borderBottom: "1px solid var(--border-default)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ fontSize: 20 }}>🌐</span>
            <div>
              <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Import from Network / Cloud URL</h2>
              <p style={{ margin: 0, fontSize: 12, color: "var(--text-secondary)" }}>
                Directly fetch remote HTTP/HTTPS/S3 assets into the project media library
              </p>
            </div>
          </div>
          <InteriorButton
            onClick={onClose}
            disabled={downloading}
            style={{
              background: "transparent",
              border: "none",
              color: "var(--text-secondary)",
              fontSize: 18,
              cursor: "pointer",
            }}
          >
            ✕
          </InteriorButton>
        </header>

        <form onSubmit={handleImport} style={{ padding: 20 }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", fontSize: 12, fontWeight: 500, marginBottom: 6, color: "var(--text-secondary)" }}>
              Network Asset URL (HTTP / HTTPS / Cloud Storage)
            </label>
            <input
              type="url"
              placeholder="https://example.com/footage/interview_cam1.mp4"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              disabled={downloading}
              required
              style={{
                width: "100%",
                padding: "8px 12px",
                background: "var(--bg-input)",
                border: "1px solid var(--border-default)",
                borderRadius: 4,
                color: "var(--text-primary)",
                fontSize: 13,
                boxSizing: "border-box",
              }}
            />
          </div>

          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", fontSize: 12, fontWeight: 500, marginBottom: 6, color: "var(--text-secondary)" }}>
              Custom Asset Name (Optional)
            </label>
            <input
              type="text"
              placeholder="e.g. Remote Interview 4K"
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              disabled={downloading}
              style={{
                width: "100%",
                padding: "8px 12px",
                background: "var(--bg-input)",
                border: "1px solid var(--border-default)",
                borderRadius: 4,
                color: "var(--text-primary)",
                fontSize: 13,
                boxSizing: "border-box",
              }}
            />
          </div>

          <div style={{ marginBottom: 16 }}>
            <span style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 6 }}>
              Quick Presets / Testing Streams:
            </span>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {sampleUrls.map((sample, i) => (
                <InteriorButton
                  key={i}
                  type="button"
                  onClick={() => {
                    setUrl(sample.url);
                    setCustomName(sample.label);
                  }}
                  disabled={downloading}
                  style={{
                    background: "var(--bg-elevated)",
                    border: "none",
                    color: "var(--text-secondary)",
                    fontSize: 11,
                    padding: "4px 8px",
                    borderRadius: 4,
                    cursor: "pointer",
                  }}
                >
                  {sample.label}
                </InteriorButton>
              ))}
            </div>
          </div>

          {error && (
            <div
              style={{
                padding: "10px 14px",
                background: "color-mix(in srgb, var(--status-danger) 20%, transparent)",
                border: "1px solid var(--status-danger)",
                borderRadius: 6,
                color: "var(--status-danger)",
                fontSize: 12,
                marginBottom: 16,
              }}
            >
              ⚠️ {error}
            </div>
          )}

          {downloading && (
            <div style={{ marginBottom: 16 }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, color: "var(--accent-text)", marginBottom: 4 }}>
                <span>{progressText}</span>
                <span>Downloading…</span>
              </div>
              <div style={{ height: 4, background: "var(--border-default)", borderRadius: 2, overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: "100%",
                    background: "var(--accent-primary)",
                    animation: "pulse 1.5s infinite",
                  }}
                />
              </div>
            </div>
          )}

          <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 20 }}>
            <InteriorButton
              type="button"
              onClick={onClose}
              disabled={downloading}
              style={{
                padding: "8px 16px",
                background: "var(--bg-elevated)",
                border: "none",
                borderRadius: 4,
                color: "var(--text-primary)",
                fontSize: 13,
                cursor: "pointer",
              }}
            >
              Cancel
            </InteriorButton>
            <InteriorButton
              type="submit"
              disabled={downloading || !url.trim()}
              style={{
                padding: "8px 18px",
                background: downloading ? "var(--bg-elevated)" : "var(--accent-primary)",
                border: "none",
                borderRadius: 4,
                color: "var(--text-on-accent)",
                fontWeight: 600,
                fontSize: 13,
                cursor: downloading ? "not-allowed" : "pointer",
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              {downloading ? "Downloading…" : "📥 Download & Import"}
            </InteriorButton>
          </div>
        </form>
      </div>
    </div>
  );
};
