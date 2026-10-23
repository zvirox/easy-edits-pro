import { InteriorButton } from "../../components/interior/interior-button";
import type { AudioClockMaster } from "../../engine/playbackEngine";

export interface LoudnessReport {
  integratedLufs: number;
  shortTermLufs: number;
  momentaryLufs: number;
  loudnessRangeLu: number;
  truePeakDbtp: number;
  targetLufs: number;
  isCompliant: boolean;
  gainRecommendationDb: number;
}

interface LoudnessPanelProps {
  loudnessStandard: "ebu_r128" | "streaming" | "atsc_a85";
  setLoudnessStandard: (val: "ebu_r128" | "streaming" | "atsc_a85") => void;
  loudnessReport: LoudnessReport;
  autoTrimLoudness: () => void;
  resetLoudness: () => void;
  audioClockMaster: AudioClockMaster;
}

export function LoudnessPanel({
  loudnessStandard,
  setLoudnessStandard,
  loudnessReport,
  autoTrimLoudness,
  resetLoudness,
  audioClockMaster,
}: LoudnessPanelProps) {
  return (
    <div className="audio-master-loudness-card">
      <div className="loudness-card-header">
        <div className="loudness-title">
          <strong>EBU R128 LOUDNESS METER</strong>
          <span className="loudness-badge">ITU-R BS.1770-4</span>
        </div>
        <div className="loudness-standard-tabs">
          <InteriorButton
            type="button"
            className={`loudness-tab-btn ${loudnessStandard === "ebu_r128" ? "active" : ""}`}
            onClick={() => {
              setLoudnessStandard("ebu_r128");
              audioClockMaster.setLoudnessStandard("ebu_r128");
            }}
            title="EBU R128: -23.0 LUFS, Max TP -1.0 dBTP"
          >
            EBU R128
          </InteriorButton>
          <InteriorButton
            type="button"
            className={`loudness-tab-btn ${loudnessStandard === "streaming" ? "active" : ""}`}
            onClick={() => {
              setLoudnessStandard("streaming");
              audioClockMaster.setLoudnessStandard("streaming");
            }}
            title="Streaming: -14.0 LUFS (YouTube, Spotify, Apple)"
          >
            Streaming (-14)
          </InteriorButton>
          <InteriorButton
            type="button"
            className={`loudness-tab-btn ${loudnessStandard === "atsc_a85" ? "active" : ""}`}
            onClick={() => {
              setLoudnessStandard("atsc_a85");
              audioClockMaster.setLoudnessStandard("atsc_a85");
            }}
            title="ATSC A/85: -24.0 LUFS, Max TP -2.0 dBTP"
          >
            ATSC A/85
          </InteriorButton>
        </div>
      </div>

      <div className="loudness-card-body">
        {/* Big Integrated LUFS Readout + Compliance Badge */}
        <div className="loudness-main-readout-row">
          <div className="integrated-display">
            <span className="lufs-value">
              {loudnessReport.integratedLufs <= -69 ? "-∞" : loudnessReport.integratedLufs.toFixed(1)}
            </span>
            <span className="lufs-unit">LUFS (Int)</span>
          </div>
          <div className="compliance-column">
            <span className={`compliance-badge ${loudnessReport.isCompliant ? "compliant" : loudnessReport.integratedLufs > -60 ? "non-compliant" : "standby"}`}>
              {loudnessReport.isCompliant ? "✓ COMPLIANT" : loudnessReport.integratedLufs > -60 ? "✕ OUT OF SPEC" : "WAITING"}
            </span>
            <span className="target-label">
              Target: <b>{loudnessReport.targetLufs.toFixed(1)} LUFS</b>
            </span>
          </div>
        </div>

        {/* Momentary & Short-term Visual Bars */}
        <div className="loudness-meters-section">
          <div className="loudness-meter-row">
            <span className="meter-label">M (400ms)</span>
            <div className="loudness-bar-track">
              <div
                className="loudness-bar-fill momentary"
                style={{ width: `${Math.max(0, Math.min(100, (loudnessReport.momentaryLufs + 60) * 2))}%` }}
              />
              <div
                className="loudness-target-marker"
                style={{ left: `${Math.max(0, Math.min(100, (loudnessReport.targetLufs + 60) * 2))}%` }}
                title={`Target: ${loudnessReport.targetLufs} LUFS`}
              />
            </div>
            <span className="meter-readout">
              {loudnessReport.momentaryLufs <= -69 ? "-∞" : `${loudnessReport.momentaryLufs.toFixed(1)}`}
            </span>
          </div>

          <div className="loudness-meter-row">
            <span className="meter-label">S (3.0s)</span>
            <div className="loudness-bar-track">
              <div
                className="loudness-bar-fill short-term"
                style={{ width: `${Math.max(0, Math.min(100, (loudnessReport.shortTermLufs + 60) * 2))}%` }}
              />
              <div
                className="loudness-target-marker"
                style={{ left: `${Math.max(0, Math.min(100, (loudnessReport.targetLufs + 60) * 2))}%` }}
                title={`Target: ${loudnessReport.targetLufs} LUFS`}
              />
            </div>
            <span className="meter-readout">
              {loudnessReport.shortTermLufs <= -69 ? "-∞" : `${loudnessReport.shortTermLufs.toFixed(1)}`}
            </span>
          </div>
        </div>

        {/* Secondary Metrics: LRA & True Peak & Trim */}
        <div className="loudness-metrics-grid">
          <div className="metric-box">
            <span className="metric-title">LRA (Range)</span>
            <b className="metric-val">{loudnessReport.loudnessRangeLu.toFixed(1)} LU</b>
          </div>
          <div className="metric-box">
            <span className="metric-title">True Peak</span>
            <b className={`metric-val ${loudnessReport.truePeakDbtp > (loudnessStandard === "atsc_a85" ? -2.0 : -1.0) ? "tp-warn" : ""}`}>
              {loudnessReport.truePeakDbtp <= -59 ? "-∞" : `${loudnessReport.truePeakDbtp.toFixed(1)} dBTP`}
            </b>
          </div>
          <div className="metric-box">
            <span className="metric-title">Trim Rec</span>
            <b className="metric-val trim-val">
              {loudnessReport.gainRecommendationDb > 0 ? "+" : ""}
              {loudnessReport.gainRecommendationDb.toFixed(1)} dB
            </b>
          </div>
        </div>

        {/* Actions: 1-Click Auto Trim & Reset */}
        <div className="loudness-actions-row">
          <InteriorButton
            type="button"
            className="loudness-autotrim-btn primary"
            onClick={autoTrimLoudness}
            disabled={loudnessReport.integratedLufs <= -60 || loudnessReport.isCompliant}
            title="Automatically adjust Master Fader to meet compliance target"
          >
            ⚡ Auto-Trim ({loudnessReport.gainRecommendationDb > 0 ? "+" : ""}{loudnessReport.gainRecommendationDb.toFixed(1)} dB)
          </InteriorButton>
          <InteriorButton
            type="button"
            className="loudness-reset-btn"
            onClick={resetLoudness}
            title="Reset accumulated loudness history & gates"
          >
            ↺ Reset
          </InteriorButton>
        </div>
      </div>
    </div>
  );
}
