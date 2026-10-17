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
        className="modal-card network-import-modal"
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 540,
          background: "#181b22",
          border: "1px solid #2d3748",
          borderRadius: 8,
          boxShadow: "0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.4)",
          color: "#e2e8f0",
          overflow: "hidden",
        }}
      >
        <header
          style={{
            padding: "16px 20px",
            borderBottom: "1px solid #2d3748",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ fontSize: 20 }}>🌐</span>
            <div>
              <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Import from Network / Cloud URL</h2>
              <p style={{ margin: 0, fontSize: 12, color: "#a0aec0" }}>
                Directly fetch remote HTTP/HTTPS/S3 assets into the project media library
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={downloading}
            style={{
              background: "transparent",
              border: "none",
              color: "#a0aec0",
              fontSize: 18,
              cursor: "pointer",
            }}
          >
            ✕
          </button>
        </header>

        <form onSubmit={handleImport} style={{ padding: 20 }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", fontSize: 12, fontWeight: 500, marginBottom: 6, color: "#cbd5e0" }}>
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
                background: "#0d1117",
                border: "1px solid #4a5568",
                borderRadius: 4,
                color: "#f7fafc",
                fontSize: 13,
                boxSizing: "border-box",
              }}
            />
          </div>

          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", fontSize: 12, fontWeight: 500, marginBottom: 6, color: "#cbd5e0" }}>
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
                background: "#0d1117",
                border: "1px solid #4a5568",
                borderRadius: 4,
                color: "#f7fafc",
                fontSize: 13,
                boxSizing: "border-box",
              }}
            />
          </div>

          <div style={{ marginBottom: 16 }}>
            <span style={{ fontSize: 11, color: "#718096", display: "block", marginBottom: 6 }}>
              Quick Presets / Testing Streams:
            </span>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {sampleUrls.map((sample, i) => (
                <button
                  key={i}
                  type="button"
                  onClick={() => {
                    setUrl(sample.url);
                    setCustomName(sample.label);
                  }}
                  disabled={downloading}
                  style={{
                    background: "#2d3748",
                    border: "none",
                    color: "#cbd5e0",
                    fontSize: 11,
                    padding: "4px 8px",
                    borderRadius: 4,
                    cursor: "pointer",
                  }}
                >
                  {sample.label}
                </button>
              ))}
            </div>
          </div>

          {error && (
            <div
              style={{
                padding: "10px 14px",
                background: "rgba(229, 62, 62, 0.2)",
                border: "1px solid #e53e3e",
                borderRadius: 6,
                color: "#fc8181",
                fontSize: 12,
                marginBottom: 16,
              }}
            >
              ⚠️ {error}
            </div>
          )}

          {downloading && (
            <div style={{ marginBottom: 16 }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, color: "#63b3ed", marginBottom: 4 }}>
                <span>{progressText}</span>
                <span>Downloading…</span>
              </div>
              <div style={{ height: 4, background: "#2d3748", borderRadius: 2, overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: "100%",
                    background: "#3182ce",
                    animation: "pulse 1.5s infinite",
                  }}
                />
              </div>
            </div>
          )}

          <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 20 }}>
            <button
              type="button"
              onClick={onClose}
              disabled={downloading}
              style={{
                padding: "8px 16px",
                background: "#2d3748",
                border: "none",
                borderRadius: 4,
                color: "#e2e8f0",
                fontSize: 13,
                cursor: "pointer",
              }}
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={downloading || !url.trim()}
              style={{
                padding: "8px 18px",
                background: downloading ? "#4a5568" : "#3182ce",
                border: "none",
                borderRadius: 4,
                color: "#ffffff",
                fontWeight: 600,
                fontSize: 13,
                cursor: downloading ? "not-allowed" : "pointer",
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              {downloading ? "Downloading…" : "📥 Download & Import"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
