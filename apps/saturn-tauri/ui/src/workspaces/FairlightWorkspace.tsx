import { InteriorButton } from "../components/interior/interior-button";
import type { Dispatch, SetStateAction, ReactNode, PointerEvent } from "react";
import {
  formatTime,
  formatDb,
} from "../utils/formatters";
import type {
  ProjectDocument,
  MediaAsset,
  SelectedClip,
  Track,
  TrackAudioProcessing,
  BusTarget,
  AutomationMode,
} from "../types";
import type { AudioClockMaster } from "../engine/playbackEngine";
import { FairlightMixerRack } from "./FairlightMixerRack";
import { FairlightTimelineTray } from "./FairlightTimelineTray";

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

export interface FairlightWorkspaceProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  playhead: number;
  timelineZoom: number;
  timelineWidth: number;
  selectedClip: SelectedClip | null;
  setSelectedClip: (clip: SelectedClip | null) => void;
  setSelectedClips: (clips: SelectedClip[]) => void;
  selectedFairlightTrackId: number | null;
  setSelectedFairlightTrackId: (id: number) => void;
  audioProcessingDrafts: Record<number, TrackAudioProcessing>;
  setAudioProcessingDrafts: Dispatch<SetStateAction<Record<number, TrackAudioProcessing>>>;
  saveTrackAudioProcessing: (trackId: number, patch: Partial<TrackAudioProcessing>) => Promise<void>;
  trackGr: Record<number, number>;
  masterGr: number;
  vuLevels: Record<number, { left: number; right: number; peakDb: number }>;
  masterVu: { left: number; right: number; peakDb: number };
  gainDrafts: Record<number, number>;
  setGainDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackGain: (trackId: number, val: number) => Promise<void>;
  panDrafts: Record<number, number>;
  setPanDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackPan: (trackId: number, val: number) => Promise<void>;
  masterGainDraft: number;
  setMasterGainDraft: (val: number) => void;
  busGainDrafts: Record<string, number>;
  setBusGainDrafts: Dispatch<SetStateAction<Record<string, number>>>;
  loudnessStandard: "ebu_r128" | "streaming" | "atsc_a85";
  setLoudnessStandard: (val: "ebu_r128" | "streaming" | "atsc_a85") => void;
  loudnessReport: LoudnessReport;
  autoTrimLoudness: () => void;
  resetLoudness: () => void;
  programPlaying: boolean;
  toggleProgramPlayback: () => void;
  stepProgramFrame: (direction: -1 | 1) => void;
  handleShuttleKey: (key: "j" | "k" | "l") => void;
  clearAllSolos: () => Promise<void> | void;
  muteAllTracks: () => Promise<void> | void;
  toggleTrackMute: (track: Track) => Promise<void> | void;
  toggleTrackSolo: (track: Track) => Promise<void> | void;
  toggleTrackLock: (track: Track) => Promise<void> | void;
  setTrackBusRoute: (trackId: number, bus: BusTarget) => Promise<void> | void;
  setTrackAutomationMode: (trackId: number, mode: AutomationMode) => Promise<void> | void;
  addTrackPlugin: (trackId: number, pluginName: string) => Promise<void> | void;
  toggleTrackPlugin: (trackId: number, pluginId: number, enabled: boolean) => Promise<void> | void;
  zoomTimeline: (factor: number) => void;
  beginTimelineScrub: (e: PointerEvent<HTMLDivElement>) => void;
  moveTimelineScrub: (e: PointerEvent<HTMLDivElement>) => void;
  finishTimelineScrub: (e: PointerEvent<HTMLDivElement>) => void;
  renderProgramMonitor: () => ReactNode;
  audioClockMaster: AudioClockMaster;
  perform: (command: string, payload: Record<string, unknown>) => Promise<unknown>;
}

export function FairlightWorkspace({
  project,
  media,
  playhead,
  timelineZoom,
  timelineWidth,
  selectedClip,
  setSelectedClip,
  setSelectedClips,
  selectedFairlightTrackId,
  setSelectedFairlightTrackId,
  audioProcessingDrafts,
  setAudioProcessingDrafts,
  saveTrackAudioProcessing,
  trackGr,
  masterGr,
  vuLevels,
  masterVu,
  gainDrafts,
  setGainDrafts,
  saveTrackGain,
  panDrafts,
  setPanDrafts,
  saveTrackPan,
  masterGainDraft,
  setMasterGainDraft,
  busGainDrafts,
  setBusGainDrafts,
  loudnessStandard,
  setLoudnessStandard,
  loudnessReport,
  autoTrimLoudness,
  resetLoudness,
  programPlaying,
  toggleProgramPlayback,
  stepProgramFrame,
  handleShuttleKey,
  clearAllSolos,
  muteAllTracks,
  toggleTrackMute,
  toggleTrackSolo,
  toggleTrackLock,
  setTrackBusRoute,
  setTrackAutomationMode,
  addTrackPlugin,
  toggleTrackPlugin,
  zoomTimeline,
  beginTimelineScrub,
  moveTimelineScrub,
  finishTimelineScrub,
  renderProgramMonitor,
  audioClockMaster,
  perform,
}: FairlightWorkspaceProps) {
  const tracks = project?.sequence.tracks ?? [];
  const sequenceEnd = Math.max(0, ...tracks.flatMap((t) => t.clips.map((c) => c.timeline_start + c.duration)));
  const audioTracks = tracks.filter((t) => t.kind === "audio");

  const activeTrackId = selectedFairlightTrackId ?? audioTracks[0]?.id ?? tracks[0]?.id ?? 1;
  const activeTrack = tracks.find((t) => t.id === activeTrackId) ?? tracks[0];
  const activeDsp: TrackAudioProcessing = audioProcessingDrafts[activeTrackId] ?? activeTrack?.audio_processing ?? {};

  const lowGain = activeDsp.eq_low_gain ?? 0;
  const midGain = activeDsp.eq_mid_gain ?? 0;
  const highGain = activeDsp.eq_high_gain ?? 0;
  const dyLow = (lowGain / 15) * 22;
  const dyMid = (midGain / 15) * 22;
  const dyHigh = (highGain / 15) * 22;
  const eqCurvePath = `M 0 ${(35 - dyLow).toFixed(1)} C 40 ${(35 - dyLow).toFixed(1)}, 70 ${(35 - dyMid).toFixed(1)}, 120 ${(35 - dyMid).toFixed(1)} C 170 ${(35 - dyMid).toFixed(1)}, 200 ${(35 - dyHigh).toFixed(1)}, 240 ${(35 - dyHigh).toFixed(1)}`;
  const eqFillPath = `${eqCurvePath} L 240 70 L 0 70 Z`;

  const activeTrackGr = trackGr[activeTrackId] || 0;

  return (
    <main className="workspace-page audio-workspace-page">
      {/* Compact 32px Audio Studio Header */}
      <header className="workspace-sub-header audio-workspace-header" style={{ height: "32px", minHeight: "32px", padding: "0 12px" }}>
        <div className="scopes-title-group" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <strong style={{ fontSize: "11px", letterSpacing: "0.06em", color: "var(--text-secondary)" }}>FAIRLIGHT SOUND SUITE</strong>
          <div className="audio-transport-readout" style={{ padding: "2px 8px" }}>
            <span className="transport-tc-label" style={{ fontSize: "7.5px" }}>TIMECODE</span>
            <code className="transport-tc-val" style={{ fontSize: "11px" }}>{formatTime(playhead)}</code>
          </div>
        </div>

        <div className="audio-header-actions" style={{ gap: "6px" }}>
          <div className="audio-transport-buttons">
            <InteriorButton
              className="transport-btn"
              onClick={() => handleShuttleKey("j")}
              title="Shuttle Reverse (J)"
              style={{ padding: "3px 7px", fontSize: "10px" }}
            >
              ◀◀
            </InteriorButton>
            <InteriorButton
              className="transport-btn"
              onClick={() => stepProgramFrame(-1)}
              title="Previous Frame (Left Arrow)"
              style={{ padding: "3px 7px", fontSize: "10px" }}
            >
              ◀|
            </InteriorButton>
            <InteriorButton
              className={`transport-btn play-btn ${programPlaying ? "active-toggle" : ""}`}
              onClick={() => toggleProgramPlayback()}
              title="Play / Pause (Space / K)"
              style={{ padding: "3px 10px", fontSize: "10px" }}
            >
              {programPlaying ? "⏸ Pause" : "▶ Play"}
            </InteriorButton>
            <InteriorButton
              className="transport-btn"
              onClick={() => stepProgramFrame(1)}
              title="Next Frame (Right Arrow)"
              style={{ padding: "3px 7px", fontSize: "10px" }}
            >
              |▶
            </InteriorButton>
            <InteriorButton
              className="transport-btn"
              onClick={() => handleShuttleKey("l")}
              title="Shuttle Forward (L)"
              style={{ padding: "3px 7px", fontSize: "10px" }}
            >
              ▶▶
            </InteriorButton>
          </div>

          <div className="audio-matrix-global-actions" style={{ gap: "4px", paddingLeft: "8px" }}>
            <InteriorButton
              className="btn-secondary"
              onClick={() => void clearAllSolos()}
              title="Clear all soloed tracks"
              style={{ padding: "3px 8px", fontSize: "10px" }}
            >
              Clear Solos
            </InteriorButton>
            <InteriorButton
              className="btn-secondary"
              onClick={() => void muteAllTracks()}
              title="Toggle mute for all tracks"
              style={{ padding: "3px 8px", fontSize: "10px" }}
            >
              Mute All
            </InteriorButton>
          </div>
        </div>
      </header>

      {/* Fairlight 3-Column Studio Body */}
      <div className="audio-studio-body" style={{ flex: 1, minHeight: 0 }}>
        {/* Left Monitor Column: Picture Sync + Master Bus Card */}
        <section className="audio-monitor-column">
          <div className="audio-video-sync-box">
            <div className="audio-box-title">
              <span>PICTURE SYNC MONITOR</span>
              <code>{formatTime(playhead)} / {formatTime(sequenceEnd)}</code>
            </div>
            <div className="audio-sync-preview-stage">
              {renderProgramMonitor()}
            </div>
          </div>

          <div className="audio-master-bus-card">
            <div className="master-bus-header">
              <div className="master-bus-title">
                <strong>MASTER OUTPUT BUS</strong>
                <span className="bus-badge">Stereo Main · 48 kHz</span>
              </div>
              <span className={`mixer-live-badge ${programPlaying ? "playing" : "stopped"}`}>
                {programPlaying ? "LIVE" : "STANDBY"}
              </span>
            </div>

            <div className="master-bus-controls">
              {/* Dual Stereo Master VU Meter */}
              <div className="master-vu-column">
                <div className="vu-meter-pair master-vu-pair">
                  <div className="vu-bar-slot">
                    <div
                      className="vu-bar-fill left"
                      style={{ height: `${Math.min(100, (masterVu.left || 0) * 100)}%` }}
                    />
                  </div>
                  <div className="vu-bar-slot">
                    <div
                      className="vu-bar-fill right"
                      style={{ height: `${Math.min(100, (masterVu.right || 0) * 100)}%` }}
                    />
                  </div>
                </div>
                <div className="vu-db-readout">{formatDb(masterVu.peakDb)}</div>
              </div>

              {/* Master Vertical Fader */}
              <div className="master-fader-column">
                <div className="fader-scale-ticks">
                  <span>+6</span>
                  <span className="unity">0</span>
                  <span>-6</span>
                  <span>-12</span>
                  <span>-24</span>
                  <span>-48</span>
                  <span>-∞</span>
                </div>
                <div className="fader-track-container">
                  <input
                    type="range"
                    className="vertical-fader master-fader"
                    min="-60"
                    max="6"
                    step="0.5"
                    value={masterGainDraft}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setMasterGainDraft(val);
                    }}
                    onPointerUp={(e) => void perform("bus.set_gain_db", { target: "master", gain_db: Number(e.currentTarget.value) })}
                  />
                </div>
                <div className="strip-fader-footer">
                  <InteriorButton
                    type="button"
                    className="strip-unity-btn master-unity"
                    title="Reset master gain to unity 0 dB"
                    onClick={() => {
                      setMasterGainDraft(0);
                      void perform("bus.set_gain_db", { target: "master", gain_db: 0 });
                    }}
                  >
                    0 dB
                  </InteriorButton>
                  <span className="strip-gain-text master-gain-text">{formatDb(masterGainDraft)}</span>
                </div>
              </div>
            </div>
          </div>

          {/* Master True Peak Limiter Module */}
          <div className="audio-master-limiter-card">
            <div className="limiter-card-header">
              <div className="limiter-title">
                <strong>TRUE PEAK LIMITER</strong>
                <span className="limiter-badge">-0.5 dBFS CEILING</span>
              </div>
              <span className="limiter-active-pill">BRICKWALL</span>
            </div>
            <div className="limiter-card-body">
              <div className="limiter-specs">
                <span>Ceiling: <b>-0.5 dB</b></span>
                <span>Ratio: <b>20:1</b></span>
                <span>Attack: <b>1.0 ms</b></span>
              </div>
              <div className="limiter-gr-container">
                <div className="gr-label-row">
                  <span>Master GR</span>
                  <b className={masterGr < -0.1 ? "gr-active" : ""}>
                    {masterGr < -0.05 ? `${masterGr.toFixed(1)} dB` : "0.0 dB"}
                  </b>
                </div>
                <div className="gr-meter-bar-slot">
                  <div
                    className="gr-meter-bar-fill master"
                    style={{ width: `${Math.min(100, Math.max(0, -masterGr * 5))}%` }}
                  />
                </div>
              </div>
            </div>
          </div>

          {/* EBU R128 Broadcast Loudness Meter & Auto-Trim */}
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

          {/* Submix 1 / 2 and Aux FX Buses */}
          <div className="audio-submix-buses-row" style={{ display: "flex", gap: "8px", marginTop: "10px", justifyContent: "center" }}>
            {[
              { target: "submix1" as const, name: "SUB 1 (DX)" },
              { target: "submix2" as const, name: "SUB 2 (MX)" },
              { target: "aux_fx" as const, name: "AUX (FX)" },
            ].map((b) => (
              <div key={b.target} className="audio-bus-channel-card">
                <div className="bus-strip-header">
                  <strong>{b.name}</strong>
                  <span className="bus-target-tag">{b.target.toUpperCase()}</span>
                </div>
                <div className="fader-track-container" style={{ height: "100px", display: "flex", alignItems: "center" }}>
                  <input
                    type="range"
                    className="vertical-fader"
                    min="-60"
                    max="6"
                    step="0.5"
                    value={busGainDrafts[b.target] ?? 0}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setBusGainDrafts((prev) => ({ ...prev, [b.target]: val }));
                      audioClockMaster.setBusGain(b.target, val);
                    }}
                    onPointerUp={(e) => void perform("bus.set_gain_db", { target: b.target, gain_db: Number(e.currentTarget.value) })}
                  />
                </div>
                <span style={{ fontSize: "10px", fontWeight: 700, color: "var(--text-primary)" }}>
                  {formatDb(busGainDrafts[b.target] ?? 0)}
                </span>
              </div>
            ))}
          </div>
        </section>

        {/* Center Column: Dedicated Fairlight Track Audio DSP Rack */}
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
                  <small>Low Shelf · Mid Peak · High Shelf</small>
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
                  <circle cx="120" cy={35 - dyMid} r="3.5" fill="#a78bfa" />
                  <circle cx="220" cy={35 - dyHigh} r="3.5" fill="#34d399" />
                </svg>
                <div className="eq-freq-labels">
                  <span>80 Hz (Low)</span>
                  <span>1.0 kHz (Mid)</span>
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

        {/* Right Column: Full-Height Channel Strip Mixer Rack */}
        <FairlightMixerRack
          tracks={tracks}
          activeTrackId={activeTrackId}
          setSelectedFairlightTrackId={setSelectedFairlightTrackId}
          trackGr={trackGr}
          vuLevels={vuLevels}
          gainDrafts={gainDrafts}
          setGainDrafts={setGainDrafts}
          saveTrackGain={saveTrackGain}
          panDrafts={panDrafts}
          setPanDrafts={setPanDrafts}
          saveTrackPan={saveTrackPan}
          saveTrackAudioProcessing={saveTrackAudioProcessing}
          setTrackBusRoute={setTrackBusRoute}
          setTrackAutomationMode={setTrackAutomationMode}
          addTrackPlugin={addTrackPlugin}
          toggleTrackPlugin={toggleTrackPlugin}
          toggleTrackMute={toggleTrackMute}
          toggleTrackSolo={toggleTrackSolo}
          toggleTrackLock={toggleTrackLock}
          audioClockMaster={audioClockMaster}
        />
      </div>

      {/* Bottom Row (220px): Audio Sequence Timeline & Waveforms */}
      <FairlightTimelineTray
        tracks={tracks}
        media={media}
        playhead={playhead}
        timelineZoom={timelineZoom}
        timelineWidth={timelineWidth}
        selectedClip={selectedClip}
        setSelectedClip={setSelectedClip}
        setSelectedClips={setSelectedClips}
        zoomTimeline={zoomTimeline}
        beginTimelineScrub={beginTimelineScrub}
        moveTimelineScrub={moveTimelineScrub}
        finishTimelineScrub={finishTimelineScrub}
        toggleTrackMute={toggleTrackMute}
        toggleTrackSolo={toggleTrackSolo}
      />
    </main>
  );
}
