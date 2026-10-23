import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction } from "react";
import type { Track, TrackAudioProcessing } from "../../types";
import type { AudioClockMaster } from "../../engine/playbackEngine";

interface DspRackPanelProps {
  tracks: Track[];
  activeTrackId: number;
  trackGr: Record<number, number>;
  audioProcessingDrafts: Record<number, TrackAudioProcessing>;
  setAudioProcessingDrafts: Dispatch<SetStateAction<Record<number, TrackAudioProcessing>>>;
  saveTrackAudioProcessing: (trackId: number, patch: Partial<TrackAudioProcessing>) => Promise<void>;
  audioClockMaster: AudioClockMaster;
  setSelectedFairlightTrackId: (id: number) => void;
}

export function DspRackPanel({
  tracks,
  activeTrackId,
  trackGr,
  audioProcessingDrafts,
  setAudioProcessingDrafts,
  saveTrackAudioProcessing,
  audioClockMaster,
  setSelectedFairlightTrackId,
}: DspRackPanelProps) {
  const activeTrack = tracks.find((t) => t.id === activeTrackId) ?? tracks[0];
  const activeDsp: TrackAudioProcessing = audioProcessingDrafts[activeTrackId] ?? activeTrack?.audio_processing ?? {};

  const lowGain = activeDsp.eq_low_gain ?? 0;
  const midGain = activeDsp.eq_mid_gain ?? 0;
  const himidGain = activeDsp.eq_highmid_gain ?? 0;
  const highGain = activeDsp.eq_high_gain ?? 0;
  const toDy = (gain: number) => (gain / 15) * 22;
  const dyLow = toDy(lowGain);
  const dyMid = toDy(midGain);
  const dyHimid = toDy(himidGain);
  const dyHigh = toDy(highGain);
  // Piecewise curve through the four band anchors (80 Hz, 1 kHz, 2.5 kHz,
  // 8 kHz) laid across the 240-unit viewBox at x = 0/80/160/240.
  const bandCurve = (
    y0: number, y1: number, y2: number, y3: number,
  ) => {
    const a = (35 - y0).toFixed(1);
    const b = (35 - y1).toFixed(1);
    const c = (35 - y2).toFixed(1);
    const d = (35 - y3).toFixed(1);
    return `M 0 ${a} C 27 ${a}, 53 ${b}, 80 ${b} C 107 ${b}, 133 ${c}, 160 ${c} C 187 ${c}, 213 ${d}, 240 ${d}`;
  };
  const eqCurvePath = bandCurve(dyLow, dyMid, dyHimid, dyHigh);
  const eqFillPath = `${eqCurvePath} L 240 70 L 0 70 Z`;

  const activeTrackGr = trackGr[activeTrackId] || 0;

  return (
    <section className="audio-dsp-rack-column">
      <div className="dsp-rack-header">
        <div className="dsp-header-left">
          <span className="dsp-eyebrow">TRACK DSP PROCESSOR</span>
          <h2>{activeTrack?.name ?? "Track"} Audio Rack</h2>
        </div>
        <div className="dsp-track-picker">
          <span className="picker-label">Target:</span>
          <select
            value={activeTrackId}
            onChange={(e) => setSelectedFairlightTrackId(Number(e.target.value))}
          >
            {tracks.map((t) => (
              <option key={t.id} value={t.id}>
                {t.kind === "audio" ? "♪ " : "▶ "} {t.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="dsp-modules-scroll">
        {/* Module 1: 4-Band Parametric Equalizer */}
        <div className={`dsp-module-card eq-module ${activeDsp.eq_enabled ? "module-enabled" : "module-bypassed"}`}>
          <div className="dsp-module-top">
            <div className="module-title-box">
              <span className="module-badge eq">EQ</span>
              <strong>4-Band Parametric Equalizer</strong>
              <small>Low Shelf · Mid Bell · High-Mid Bell · High Shelf</small>
            </div>
            <InteriorButton
              type="button"
              className={`dsp-power-btn ${activeDsp.eq_enabled ? "active" : ""}`}
              onClick={() => void saveTrackAudioProcessing(activeTrackId, { eq_enabled: !activeDsp.eq_enabled })}
            >
              {activeDsp.eq_enabled ? "ACTIVE" : "BYPASS"}
            </InteriorButton>
          </div>

          {/* Live EQ Curve Graph */}
          <div className="eq-curve-display">
            <svg viewBox="0 0 240 70" preserveAspectRatio="none" className="eq-curve-svg">
              <defs>
                <linearGradient id="eqGrad" x1="0%" y1="0%" x2="0%" y2="100%">
                  <stop offset="0%" stopColor="#6b21a8" stopOpacity="0.35" />
                  <stop offset="100%" stopColor="#6b21a8" stopOpacity="0.0" />
                </linearGradient>
              </defs>
              <line x1="0" y1="35" x2="240" y2="35" stroke="#462d40" strokeDasharray="3 3" strokeWidth="1" />
              <path d={eqFillPath} fill="url(#eqGrad)" />
              <path d={eqCurvePath} fill="none" stroke={activeDsp.eq_enabled ? "#d9468f" : "#7d6377"} strokeWidth="2.2" strokeLinecap="round" />
              <circle cx="20" cy={35 - dyLow} r="3.5" fill="#be185d" />
              <circle cx="80" cy={35 - dyMid} r="3.5" fill="#a78bfa" />
              <circle cx="160" cy={35 - dyHimid} r="3.5" fill="#f59e0b" />
              <circle cx="220" cy={35 - dyHigh} r="3.5" fill="#34d399" />
            </svg>
            <div className="eq-freq-labels">
              <span>80 Hz (Low)</span>
              <span>1.0 kHz (Mid)</span>
              <span>2.5 kHz (Hi-Mid)</span>
              <span>8.0 kHz (High)</span>
            </div>
          </div>

          <div className="eq-bands-controls">
            <div className="eq-band-item">
              <div className="band-header">
                <span>Low Shelf (80 Hz)</span>
                <b>{lowGain > 0 ? `+${lowGain.toFixed(1)}` : lowGain.toFixed(1)} dB</b>
              </div>
              <input
                type="range"
                min="-15"
                max="15"
                step="0.5"
                value={lowGain}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_low_gain: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_low_gain: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_low_gain: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_low_gain: Number(e.currentTarget.value) })}
              />
            </div>

            <div className="eq-band-item">
              <div className="band-header">
                <span>Mid Bell (1 kHz)</span>
                <b>{midGain > 0 ? `+${midGain.toFixed(1)}` : midGain.toFixed(1)} dB</b>
              </div>
              <input
                type="range"
                min="-15"
                max="15"
                step="0.5"
                value={midGain}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_mid_gain: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_mid_gain: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_mid_gain: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_mid_gain: Number(e.currentTarget.value) })}
              />
            </div>

            <div className="eq-band-item">
              <div className="band-header">
                <span>High-Mid Bell (2.5 kHz)</span>
                <b>{himidGain > 0 ? `+${himidGain.toFixed(1)}` : himidGain.toFixed(1)} dB</b>
              </div>
              <input
                type="range"
                min="-15"
                max="15"
                step="0.5"
                value={himidGain}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_highmid_gain: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_highmid_gain: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_highmid_gain: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_highmid_gain: Number(e.currentTarget.value) })}
              />
            </div>

            <div className="eq-band-item">
              <div className="band-header">
                <span>High Shelf (8 kHz)</span>
                <b>{highGain > 0 ? `+${highGain.toFixed(1)}` : highGain.toFixed(1)} dB</b>
              </div>
              <input
                type="range"
                min="-15"
                max="15"
                step="0.5"
                value={highGain}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_high_gain: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_high_gain: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_high_gain: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_high_gain: Number(e.currentTarget.value) })}
              />
            </div>
          </div>
        </div>

        {/* Module 2: Dynamics Compressor */}
        <div className={`dsp-module-card comp-module ${activeDsp.comp_enabled ? "module-enabled" : "module-bypassed"}`}>
          <div className="dsp-module-top">
            <div className="module-title-box">
              <span className="module-badge comp">DYN</span>
              <strong>Dynamics Compressor</strong>
              <small>Threshold · Ratio · Attack · Release</small>
            </div>
            <InteriorButton
              type="button"
              className={`dsp-power-btn comp ${activeDsp.comp_enabled ? "active" : ""}`}
              onClick={() => void saveTrackAudioProcessing(activeTrackId, { comp_enabled: !activeDsp.comp_enabled })}
            >
              {activeDsp.comp_enabled ? "ACTIVE" : "BYPASS"}
            </InteriorButton>
          </div>

          {/* Real-Time Gain Reduction (GR) Meter */}
          <div className="comp-gr-meter-row">
            <div className="gr-meta">
              <span className="gr-title">GAIN REDUCTION (GR)</span>
              <b className={activeTrackGr < -0.1 ? "gr-active" : ""}>
                {activeTrackGr < -0.05 ? `${activeTrackGr.toFixed(1)} dB` : "0.0 dB"}
              </b>
            </div>
            <div className="gr-meter-bar-slot">
              <div
                className="gr-meter-bar-fill"
                style={{ width: `${Math.min(100, Math.max(0, -activeTrackGr * 4.16))}%` }}
              />
            </div>
            <div className="gr-scale-marks">
              <span>0</span>
              <span>-6</span>
              <span>-12</span>
              <span>-18</span>
              <span>-24 dB</span>
            </div>
          </div>

          <div className="comp-params-grid">
            <div className="comp-param-item">
              <div className="param-label-row">
                <span>Threshold</span>
                <b>{(activeDsp.comp_threshold ?? -24).toFixed(0)} dB</b>
              </div>
              <input
                type="range"
                min="-60"
                max="0"
                step="1"
                value={activeDsp.comp_threshold ?? -24}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_threshold: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_threshold: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_threshold: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_threshold: Number(e.currentTarget.value) })}
              />
            </div>

            <div className="comp-param-item">
              <div className="param-label-row">
                <span>Ratio</span>
                <b>{(activeDsp.comp_ratio ?? 4).toFixed(1)}:1</b>
              </div>
              <input
                type="range"
                min="1"
                max="20"
                step="0.5"
                value={activeDsp.comp_ratio ?? 4}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_ratio: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_ratio: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_ratio: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_ratio: Number(e.currentTarget.value) })}
              />
            </div>

            <div className="comp-param-item">
              <div className="param-label-row">
                <span>Attack</span>
                <b>{Math.round((activeDsp.comp_attack ?? 0.02) * 1000)} ms</b>
              </div>
              <input
                type="range"
                min="1"
                max="200"
                step="1"
                value={Math.round((activeDsp.comp_attack ?? 0.02) * 1000)}
                onChange={(e) => {
                  const val = Number(e.target.value) / 1000;
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_attack: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_attack: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_attack: Number(e.currentTarget.value) / 1000 })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_attack: Number(e.currentTarget.value) / 1000 })}
              />
            </div>

            <div className="comp-param-item">
              <div className="param-label-row">
                <span>Release</span>
                <b>{Math.round((activeDsp.comp_release ?? 0.25) * 1000)} ms</b>
              </div>
              <input
                type="range"
                min="20"
                max="1000"
                step="10"
                value={Math.round((activeDsp.comp_release ?? 0.25) * 1000)}
                onChange={(e) => {
                  const val = Number(e.target.value) / 1000;
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_release: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_release: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_release: Number(e.currentTarget.value) / 1000 })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_release: Number(e.currentTarget.value) / 1000 })}
              />
            </div>
          </div>
        </div>

        {/* Module 3: Vocal De-Noise & Clean-Up */}
        <div className={`dsp-module-card denoise-module ${activeDsp.denoise_enabled ? "module-enabled" : "module-bypassed"}`}>
          <div className="dsp-module-top">
            <div className="module-title-box">
              <span className="module-badge denoise">NR</span>
              <strong>Vocal De-Noise &amp; Dialogue Isolation</strong>
              <small>80 Hz High-Pass Sub Cut · Real-Time Spectral Clean</small>
            </div>
            <InteriorButton
              type="button"
              className={`dsp-power-btn denoise ${activeDsp.denoise_enabled ? "active" : ""}`}
              onClick={() => void saveTrackAudioProcessing(activeTrackId, { denoise_enabled: !activeDsp.denoise_enabled })}
            >
              {activeDsp.denoise_enabled ? "ACTIVE" : "BYPASS"}
            </InteriorButton>
          </div>

          <div className="denoise-controls-row">
            <div className="denoise-rumble-badge">
              <span className="rumble-icon">⚡</span>
              <div className="rumble-text">
                <strong>80 Hz High-Pass Rumble Filter</strong>
                <small>Removes low-end mic bumps &amp; sub-bass rumble</small>
              </div>
              <span className={`rumble-status ${activeDsp.denoise_enabled ? "engaged" : "off"}`}>
                {activeDsp.denoise_enabled ? "ENGAGED" : "OFF"}
              </span>
            </div>

            <div className="denoise-amount-slider-item">
              <div className="slider-header-row">
                <span>Noise Suppression Intensity</span>
                <b>{Math.round((activeDsp.denoise_amount ?? 0.5) * 100)}%</b>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                disabled={!activeDsp.denoise_enabled}
                value={activeDsp.denoise_amount ?? 0.5}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, denoise_amount: val } }));
                  audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, denoise_amount: val });
                }}
                onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { denoise_amount: Number(e.currentTarget.value) })}
                onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { denoise_amount: Number(e.currentTarget.value) })}
              />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
