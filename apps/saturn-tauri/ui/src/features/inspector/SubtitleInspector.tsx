import { formatTime } from "../../utils/formatters";
import type { SubtitleItem } from "../../app/projectTypes";
import { TICKS_PER_SECOND } from "../../engine/playback/constants";

export interface SubtitleInspectorProps {
  selectedSubtitle: SubtitleItem;
  setSelectedSubtitle: (sub: SubtitleItem | null) => void;
  subtitleTextDraft: string | null;
  setSubtitleTextDraft: (text: string | null) => void;
  updateSubtitle: (id: number, patch: Partial<SubtitleItem>) => Promise<void>;
  perform: (cmd: string, params?: unknown) => Promise<unknown>;
  removeSubtitle: (id: number) => Promise<void>;
  importSubtitlesDialog: () => Promise<void>;
  exportSubtitlesDialog: () => Promise<void>;
  clearSubtitles: () => Promise<void>;
  setTranscribeModalOpen: (open: boolean) => void;
  handleDetectSilence: () => Promise<void>;
  setSilenceModalOpen: (open: boolean) => void;
}

export function SubtitleInspector({
  selectedSubtitle,
  setSelectedSubtitle,
  subtitleTextDraft,
  setSubtitleTextDraft,
  updateSubtitle,
  perform,
  removeSubtitle,
  importSubtitlesDialog,
  exportSubtitlesDialog,
  clearSubtitles,
  setTranscribeModalOpen,
  handleDetectSilence,
  setSilenceModalOpen,
}: SubtitleInspectorProps) {
  const durationSec = (selectedSubtitle.end_ticks - selectedSubtitle.start_ticks) / TICKS_PER_SECOND;

  return (
    <div className="subtitle-inspector">
      <div className="subtitle-inspector-header">
        <h2>Subtitle Inspector</h2>
        <button
          type="button"
          className="inspector-close-btn"
          onClick={() => setSelectedSubtitle(null)}
          title="Close Subtitle Inspector"
        >
          ×
        </button>
      </div>
      <p>Caption text and millisecond timecodes for card #{selectedSubtitle.id}.</p>

      <label className="subtitle-textarea-label">
        <span>Caption Text</span>
        <textarea
          rows={4}
          value={subtitleTextDraft !== null ? subtitleTextDraft : selectedSubtitle.text}
          placeholder="Enter subtitle caption text…"
          onChange={(e) => setSubtitleTextDraft(e.target.value)}
          onBlur={(e) => {
            const val = e.target.value;
            void updateSubtitle(selectedSubtitle.id, { text: val });
            setSubtitleTextDraft(null);
          }}
        />
      </label>

      <div className="subtitle-tc-cards">
        <div className="subtitle-tc-card">
          <span className="subtitle-tc-title">START TIME</span>
          <code className="subtitle-tc-value">{formatTime(selectedSubtitle.start_ticks)}</code>
          <div className="subtitle-nudge-row">
            <button
              type="button"
              className="btn-tiny"
              onClick={() => {
                const newStart = Math.max(0, selectedSubtitle.start_ticks - Math.round(0.5 * TICKS_PER_SECOND));
                void updateSubtitle(selectedSubtitle.id, { start_ticks: newStart });
              }}
              title="Nudge 0.5s Earlier"
            >
              -0.5s
            </button>
            <button
              type="button"
              className="btn-tiny"
              onClick={() => {
                const newStart = Math.min(
                  selectedSubtitle.end_ticks - Math.round(0.2 * TICKS_PER_SECOND),
                  selectedSubtitle.start_ticks + Math.round(0.5 * TICKS_PER_SECOND)
                );
                void updateSubtitle(selectedSubtitle.id, { start_ticks: newStart });
              }}
              title="Nudge 0.5s Later"
            >
              +0.5s
            </button>
          </div>
        </div>

        <div className="subtitle-tc-card">
          <span className="subtitle-tc-title">END TIME</span>
          <code className="subtitle-tc-value">{formatTime(selectedSubtitle.end_ticks)}</code>
          <div className="subtitle-nudge-row">
            <button
              type="button"
              className="btn-tiny"
              onClick={() => {
                const newEnd = Math.max(
                  selectedSubtitle.start_ticks + Math.round(0.2 * TICKS_PER_SECOND),
                  selectedSubtitle.end_ticks - Math.round(0.5 * TICKS_PER_SECOND)
                );
                void updateSubtitle(selectedSubtitle.id, { end_ticks: newEnd });
              }}
              title="Nudge 0.5s Earlier"
            >
              -0.5s
            </button>
            <button
              type="button"
              className="btn-tiny"
              onClick={() => {
                const newEnd = selectedSubtitle.end_ticks + Math.round(0.5 * TICKS_PER_SECOND);
                void updateSubtitle(selectedSubtitle.id, { end_ticks: newEnd });
              }}
              title="Nudge 0.5s Later"
            >
              +0.5s
            </button>
          </div>
        </div>
      </div>

      <div className="control">
        Duration{" "}
        <span>
          {durationSec.toFixed(2)}s ({formatTime(selectedSubtitle.end_ticks - selectedSubtitle.start_ticks)})
        </span>
      </div>

      <div className="subtitle-action-buttons">
        <button
          type="button"
          className="btn-secondary"
          onClick={() => {
            const start = selectedSubtitle.end_ticks;
            const end = start + 3 * TICKS_PER_SECOND;
            void perform("subtitle.add", { start_ticks: start, end_ticks: end, text: "Next Subtitle" });
          }}
        >
          ＋ Add Subtitle Next
        </button>
        <button
          type="button"
          className="danger-button"
          onClick={() => void removeSubtitle(selectedSubtitle.id)}
        >
          🗑 Delete Subtitle
        </button>
      </div>

      <div className="subtitle-io-buttons">
        <button
          type="button"
          className="btn-secondary"
          onClick={() => void importSubtitlesDialog()}
        >
          📥 Import .SRT/.VTT
        </button>
        <button
          type="button"
          className="btn-secondary"
          onClick={() => void exportSubtitlesDialog()}
        >
          📤 Export .SRT/.VTT
        </button>
      </div>

      <button
        type="button"
        className="danger-button"
        style={{ marginTop: "4px" }}
        onClick={() => void clearSubtitles()}
        title="Delete all subtitles in the project"
      >
        🗑 Clear All Subtitles
      </button>

      <div
        className="subtitle-ai-buttons"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "6px",
          marginTop: "8px",
          paddingTop: "8px",
          borderTop: "1px solid var(--border-color)",
        }}
      >
        <button
          type="button"
          className="primary"
          style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "6px" }}
          onClick={() => setTranscribeModalOpen(true)}
          title="Transcribe speech with offline Whisper AI directly into Subtitle track"
        >
          <span>🎙️ Auto-Transcribe (Whisper AI)</span>
        </button>
        <button
          type="button"
          className="btn-secondary"
          style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "6px" }}
          onClick={() => {
            void handleDetectSilence();
            setSilenceModalOpen(true);
          }}
          title="Detect and ripple-delete silence intervals and verbal hesitation fillers"
        >
          <span>✂️ Remove Silence &amp; Fillers</span>
        </button>
      </div>
    </div>
  );
}
