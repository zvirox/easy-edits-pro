export interface SilenceRemovalModalProps {
  isOpen: boolean;
  onClose: () => void;
  silenceThresholdDb: number;
  setSilenceThresholdDb: (v: number) => void;
  silenceMinDurationMs: number;
  setSilenceMinDurationMs: (v: number) => void;
  silencePaddingMs: number;
  setSilencePaddingMs: (v: number) => void;
  silenceIntervalsLength: number;
  isDetectingSilence: boolean;
  handleDetectSilence: () => Promise<void>;
  handleRippleDeleteFillers: () => Promise<void>;
  handleRippleDeleteSilence: () => Promise<void>;
  busy: boolean;
}

export function SilenceRemovalModal({
  isOpen,
  onClose,
  silenceThresholdDb,
  setSilenceThresholdDb,
  silenceMinDurationMs,
  setSilenceMinDurationMs,
  silencePaddingMs,
  setSilencePaddingMs,
  silenceIntervalsLength,
  isDetectingSilence,
  handleDetectSilence,
  handleRippleDeleteFillers,
  handleRippleDeleteSilence,
  busy,
}: SilenceRemovalModalProps) {
  if (!isOpen) return null;

  return (
    <div className="dialog-backdrop" role="presentation">
      <div className="modal-dialog-card speech-modal" style={{ maxWidth: "540px" }}>
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h2 style={{ margin: 0, fontSize: "16px" }}>✂️ Automated Silence &amp; Filler Word Removal</h2>
          <button type="button" className="close-btn" onClick={onClose}>✕</button>
        </div>
        <p className="modal-desc" style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "14px" }}>
          Detects low-amplitude silence gaps and verbal hesitation fillers across timeline tracks, performing multi-track ripple deletion with automatic gap closing.
        </p>
        <div style={{ display: "flex", flexDirection: "column", gap: "10px", marginBottom: "14px" }}>
          <label style={{ fontSize: "11px", display: "flex", justifyContent: "space-between" }}>
            <span>Silence Threshold ({silenceThresholdDb} dBFS)</span>
            <input
              type="range"
              min="-60"
              max="-15"
              step="1"
              value={silenceThresholdDb}
              onChange={(e) => setSilenceThresholdDb(Number(e.target.value))}
            />
          </label>
          <label style={{ fontSize: "11px", display: "flex", justifyContent: "space-between" }}>
            <span>Min Silence Duration ({silenceMinDurationMs} ms)</span>
            <input
              type="range"
              min="200"
              max="1500"
              step="50"
              value={silenceMinDurationMs}
              onChange={(e) => setSilenceMinDurationMs(Number(e.target.value))}
            />
          </label>
          <label style={{ fontSize: "11px", display: "flex", justifyContent: "space-between" }}>
            <span>Edge Padding ({silencePaddingMs} ms)</span>
            <input
              type="range"
              min="10"
              max="200"
              step="10"
              value={silencePaddingMs}
              onChange={(e) => setSilencePaddingMs(Number(e.target.value))}
            />
          </label>
        </div>

        <div style={{ background: "rgba(255, 255, 255, 0.04)", border: "1px solid var(--border-color)", borderRadius: "6px", padding: "10px", display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
          <div>
            <strong style={{ fontSize: "14px", color: "#38bdf8" }}>{silenceIntervalsLength} Pauses</strong>
            <span style={{ fontSize: "11px", color: "var(--text-muted)", marginLeft: "8px" }}>· 2 Verbal Fillers (&quot;um&quot;, &quot;like&quot;)</span>
          </div>
          <button
            type="button"
            className="btn-secondary"
            style={{ fontSize: "11px", padding: "4px 8px" }}
            onClick={() => void handleDetectSilence()}
            disabled={isDetectingSilence}
          >
            {isDetectingSilence ? "Scanning…" : "🔄 Re-Analyze"}
          </button>
        </div>

        <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
          <button type="button" onClick={onClose}>Cancel</button>
          <button
            type="button"
            className="btn-secondary"
            onClick={() => void handleRippleDeleteFillers()}
            disabled={busy}
          >
            ✂️ Ripple Remove Fillers
          </button>
          <button
            type="button"
            className="primary"
            onClick={() => void handleRippleDeleteSilence()}
            disabled={busy}
          >
            ✂️ Ripple Delete Pauses ({silenceIntervalsLength})
          </button>
        </div>
      </div>
    </div>
  );
}
