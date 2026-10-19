import { InteriorButton } from "./interior/interior-button";
import { type Dispatch, type SetStateAction, useMemo } from "react";
import type { Track, ProjectDocument, Clip, MediaAsset } from "../types";
import type { AudioClockMaster } from "../engine/playbackEngine";
import { formatDb, getClipWaveform } from "../utils/formatters";

export interface AudioMonitorMixerProps {
  project: ProjectDocument["project"] | null;
  masterVu: { left: number; right: number; peakDb: number };
  vuLevels: Record<number, { left: number; right: number; peakDb: number }>;
  loudnessReport?: {
    integratedLufs: number;
    shortTermLufs: number;
    momentaryLufs: number;
    targetLufs: number;
    isCompliant: boolean;
  };
  masterGainDraft: number;
  setMasterGainDraft: (val: number) => void;
  gainDrafts: Record<number, number>;
  setGainDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackGain: (trackId: number, val: number) => Promise<void>;
  toggleTrackMute: (track: Track) => Promise<void> | void;
  toggleTrackSolo: (track: Track) => Promise<void> | void;
  audioClockMaster: AudioClockMaster;
  activeClip?: { track: Track; clip: Clip; media: MediaAsset } | Clip | null;
  media?: MediaAsset[];
  playhead?: number;
}

export function AudioMonitorMixer({
  project,
  masterVu,
  vuLevels,
  loudnessReport,
  masterGainDraft,
  setMasterGainDraft,
  gainDrafts,
  setGainDrafts,
  saveTrackGain,
  toggleTrackMute,
  toggleTrackSolo,
  audioClockMaster,
  activeClip,
  media,
}: AudioMonitorMixerProps) {
  const tracks = project?.sequence.tracks ?? [];

  // Active clip waveform or master fallback
  const activeMediaItem = useMemo(() => {
    if (!activeClip) return null;
    if ("media" in activeClip && activeClip.media) return activeClip.media;
    if (!media) return null;
    const clip = "clip" in activeClip ? activeClip.clip : activeClip;
    return media.find((m) => m.id === clip.media_id) ?? null;
  }, [activeClip, media]);

  const activeWaveform = useMemo(() => {
    return getClipWaveform(activeMediaItem);
  }, [activeMediaItem]);

  // Master VU levels normalized
  const leftPct = Math.min(100, Math.max(0, (masterVu.left || 0) * 100));
  const rightPct = Math.min(100, Math.max(0, (masterVu.right || 0) * 100));
  const peakDb = masterVu.peakDb ?? -60;

  const peakClass = peakDb > -0.5 ? "clip" : peakDb > -6.0 ? "warn" : "";

  return (
    <div className="audio-monitor-mixer-container">
      {/* Sub-Section 1: Live Waveform & Stereo VU Meter Strip */}
      <div className="audio-meter-waveform-strip">
        <div className="meter-strip-top">
          <span className="meter-strip-title">Master Audio Monitor</span>
          <div className="meter-strip-badges">
            {loudnessReport && (
              <span className="meter-lufs-badge" title="Integrated LUFS">
                {loudnessReport.integratedLufs <= -69
                  ? "-∞ LUFS"
                  : `${loudnessReport.integratedLufs.toFixed(1)} LUFS`}
              </span>
            )}
            <span
              className={`meter-peak-badge ${peakClass}`}
              title="Master Peak dBFS"
            >
              {formatDb(peakDb)}
            </span>
          </div>
        </div>

        {/* Live Frequency / Waveform Bar Box */}
        <div className="meter-waveform-visual" aria-hidden="true">
          <svg
            className="w-full h-full"
            viewBox="0 0 100 48"
            preserveAspectRatio="none"
          >
            <defs>
              <linearGradient id="meterWfGrad" x1="0%" y1="0%" x2="0%" y2="100%">
                <stop offset="0%" stopColor="#be185d" stopOpacity="0.8" />
                <stop offset="50%" stopColor="#10b981" stopOpacity="0.6" />
                <stop offset="100%" stopColor="#6b21a8" stopOpacity="0.1" />
              </linearGradient>
            </defs>
            {activeWaveform.map((val, idx) => {
              const x = (idx / activeWaveform.length) * 100;
              const barWidth = 100 / activeWaveform.length - 0.4;
              // Modulate by master VU level for live responsiveness
              const mod = 0.6 + (masterVu.left + masterVu.right) * 0.4;
              const height = Math.min(46, Math.max(2, (val / 255) * 44 * mod));
              const y = 24 - height / 2;
              return (
                <rect
                  key={idx}
                  x={x}
                  y={y}
                  width={Math.max(0.6, barWidth)}
                  height={height}
                  fill="url(#meterWfGrad)"
                  rx="0.5"
                />
              );
            })}
          </svg>
        </div>

        {/* Dual Stereo VU Meter */}
        <div className="stereo-vu-strip">
          <div className="stereo-vu-channel">
            <span className="vu-ch-label">L</span>
            <div className="vu-ch-track">
              <div className="vu-ch-bar" style={{ width: `${leftPct}%` }} />
            </div>
          </div>
          <div className="stereo-vu-channel">
            <span className="vu-ch-label">R</span>
            <div className="vu-ch-track">
              <div className="vu-ch-bar" style={{ width: `${rightPct}%` }} />
            </div>
          </div>
          <div className="stereo-vu-scale">
            <span>-60</span>
            <span>-36</span>
            <span>-18</span>
            <span>-6</span>
            <span>0 dB</span>
          </div>
        </div>
      </div>

      {/* Sub-Section 2: Compact Track Mixer */}
      <div className="compact-mixer-section">
        <div className="compact-mixer-title">Track Mixer</div>
        <div className="compact-mixer-strips scrollbar-thin">
          {/* Master Bus Channel Strip */}
          <div className="compact-channel-row">
            <span className="compact-ch-badge master">M</span>
            <span className="compact-ch-name" title="Master Output Bus">
              Master
            </span>
            <div className="compact-ch-btns" style={{ width: "32px" }} />
            <div className="compact-fader-container">
              <input
                type="range"
                className="compact-fader-slider"
                min="-60"
                max="6"
                step="0.5"
                value={masterGainDraft}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setMasterGainDraft(val);
                  audioClockMaster.setMasterGain(val);
                }}
                title={`Master Fader: ${formatDb(masterGainDraft)}`}
              />
            </div>
            <span className="compact-ch-db">{formatDb(masterGainDraft)}</span>
          </div>

          {/* Timeline Tracks */}
          {tracks.map((track) => {
            const currentGain = gainDrafts[track.id] ?? track.gain_db ?? 0;
            const trackVu = vuLevels[track.id] ?? { left: 0, right: 0, peakDb: -60 };
            const badgeKind = track.kind === "audio" ? "audio" : "video";
            const badgeLabel = track.kind === "audio" ? `A${track.id}` : `V${track.id}`;

            return (
              <div key={track.id} className="compact-channel-row">
                <span className={`compact-ch-badge ${badgeKind}`} title={`${track.name} (Peak: ${formatDb(trackVu.peakDb)})`}>
                  {badgeLabel}
                </span>
                <span className="compact-ch-name" title={track.name}>
                  {track.name}
                </span>

                <div className="compact-ch-btns">
                  <InteriorButton
                    type="button"
                    className={`compact-mute-btn ${track.muted ? "active" : ""}`}
                    onClick={() => void toggleTrackMute(track)}
                    title={track.muted ? "Unmute track" : "Mute track"}
                  >
                    M
                  </InteriorButton>
                  <InteriorButton
                    type="button"
                    className={`compact-solo-btn ${track.solo ? "active" : ""}`}
                    onClick={() => void toggleTrackSolo(track)}
                    title={track.solo ? "Solo active" : "Solo track"}
                  >
                    S
                  </InteriorButton>
                </div>

                <div className="compact-fader-container">
                  <input
                    type="range"
                    className="compact-fader-slider"
                    min="-60"
                    max="6"
                    step="0.5"
                    value={currentGain}
                    onChange={(e) => {
                      const val = Number(e.target.value);
                      setGainDrafts((prev) => ({ ...prev, [track.id]: val }));
                      audioClockMaster.updateTrackGain(track.id, val);
                    }}
                    onPointerUp={(e) => void saveTrackGain(track.id, Number(e.currentTarget.value))}
                    onKeyUp={(e) => void saveTrackGain(track.id, Number(e.currentTarget.value))}
                    title={`${track.name} Gain: ${formatDb(currentGain)} | Peak: ${formatDb(trackVu.peakDb)}`}
                  />
                </div>

                <span className="compact-ch-db">{formatDb(currentGain)}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
