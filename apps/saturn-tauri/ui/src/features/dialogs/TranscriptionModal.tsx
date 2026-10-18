export interface TranscriptionModalProps {
  isOpen: boolean;
  onClose: () => void;
  transcribeModel: string;
  setTranscribeModel: (m: string) => void;
  transcribeLanguage: string;
  setTranscribeLanguage: (l: string) => void;
  transcribeTextOverride: string;
  setTranscribeTextOverride: (t: string) => void;
  transcribeReplaceExisting: boolean;
  setTranscribeReplaceExisting: (r: boolean) => void;
  isTranscribing: boolean;
  handleAutoTranscribe: () => Promise<void>;
}

export function TranscriptionModal({
  isOpen,
  onClose,
  transcribeModel,
  setTranscribeModel,
  transcribeLanguage,
  setTranscribeLanguage,
  transcribeTextOverride,
  setTranscribeTextOverride,
  transcribeReplaceExisting,
  setTranscribeReplaceExisting,
  isTranscribing,
  handleAutoTranscribe,
}: TranscriptionModalProps) {
  if (!isOpen) return null;

  return (
    <div className="dialog-backdrop" role="presentation">
      <div className="modal-dialog-card speech-modal" style={{ maxWidth: "520px" }}>
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h2 style={{ margin: 0, fontSize: "16px" }}>🎙️ Auto-Speech Transcription (Whisper AI)</h2>
          <button type="button" className="close-btn" onClick={onClose}>✕</button>
        </div>
        <p className="modal-desc" style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "14px" }}>
          Generate synchronized subtitle cues directly onto your timeline's Subtitle track using embedded offline speech intelligence.
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "12px", marginBottom: "12px" }}>
          <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px" }}>
            Whisper Model
            <select
              value={transcribeModel}
              onChange={(e) => setTranscribeModel(e.target.value)}
              style={{ padding: "6px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
            >
              <option value="base">Whisper Base (Recommended)</option>
              <option value="small">Whisper Small (High Precision)</option>
              <option value="tiny">Whisper Tiny (Ultra-Fast)</option>
            </select>
          </label>
          <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px" }}>
            Spoken Language
            <select
              value={transcribeLanguage}
              onChange={(e) => setTranscribeLanguage(e.target.value)}
              style={{ padding: "6px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
            >
              <option value="en">English (en)</option>
              <option value="es">Spanish (es)</option>
              <option value="fr">French (fr)</option>
              <option value="de">German (de)</option>
              <option value="auto">Auto-Detect</option>
            </select>
          </label>
        </div>
        <label style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginBottom: "12px" }}>
          Optional Script / Voiceover Text (Overrides audio analysis)
          <textarea
            rows={3}
            placeholder="Leave blank to analyze timeline audio tracks automatically, or paste script text here..."
            value={transcribeTextOverride}
            onChange={(e) => setTranscribeTextOverride(e.target.value)}
            style={{ width: "100%", padding: "6px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px", resize: "vertical" }}
          />
        </label>
        <label style={{ fontSize: "11px", display: "flex", alignItems: "center", gap: "8px", marginBottom: "16px", cursor: "pointer" }}>
          <input
            type="checkbox"
            checked={transcribeReplaceExisting}
            onChange={(e) => setTranscribeReplaceExisting(e.target.checked)}
          />
          <span>Replace existing subtitles in current sequence</span>
        </label>
        <div className="dialog-actions" style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}>
          <button type="button" onClick={onClose}>Cancel</button>
          <button
            type="button"
            className="primary"
            onClick={() => void handleAutoTranscribe()}
            disabled={isTranscribing}
          >
            {isTranscribing ? "Transcribing Audio…" : "🎙️ Generate Subtitles"}
          </button>
        </div>
      </div>
    </div>
  );
}
