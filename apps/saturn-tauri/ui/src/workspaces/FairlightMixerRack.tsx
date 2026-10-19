import { InteriorButton } from "../components/interior/interior-button";
import type { Dispatch, SetStateAction } from "react";
import { formatDb, formatPan } from "../utils/formatters";
import type { AutomationMode, BusTarget, Track, TrackAudioProcessing } from "../types";
import type { AudioClockMaster } from "../engine/playbackEngine";

interface FairlightMixerRackProps {
  tracks: Track[];
  activeTrackId: number;
  setSelectedFairlightTrackId: (id: number) => void;
  trackGr: Record<number, number>;
  vuLevels: Record<number, { left: number; right: number; peakDb: number }>;
  gainDrafts: Record<number, number>;
  setGainDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackGain: (trackId: number, value: number) => Promise<void>;
  panDrafts: Record<number, number>;
  setPanDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackPan: (trackId: number, value: number) => Promise<void>;
  saveTrackAudioProcessing: (trackId: number, patch: Partial<TrackAudioProcessing>) => Promise<void>;
  setTrackBusRoute: (trackId: number, bus: BusTarget) => Promise<void> | void;
  setTrackAutomationMode: (trackId: number, mode: AutomationMode) => Promise<void> | void;
  addTrackPlugin: (trackId: number, name: string) => Promise<void> | void;
  toggleTrackPlugin: (trackId: number, pluginId: number, enabled: boolean) => Promise<void> | void;
  toggleTrackMute: (track: Track) => Promise<void> | void;
  toggleTrackSolo: (track: Track) => Promise<void> | void;
  toggleTrackLock: (track: Track) => Promise<void> | void;
  audioClockMaster: AudioClockMaster;
}

export function FairlightMixerRack({
  tracks, activeTrackId, setSelectedFairlightTrackId,
  trackGr, vuLevels, gainDrafts, setGainDrafts, saveTrackGain,
  panDrafts, setPanDrafts, saveTrackPan, saveTrackAudioProcessing,
  setTrackBusRoute, setTrackAutomationMode, addTrackPlugin, toggleTrackPlugin,
  toggleTrackMute, toggleTrackSolo, toggleTrackLock, audioClockMaster,
}: FairlightMixerRackProps) {
  const audioTracks = tracks.filter((track) => track.kind === "audio");
  const videoTracks = tracks.filter((track) => track.kind === "video");
  return (
        <section className="audio-mixer-rack-column">
          <div className="mixer-rack-toolbar">
            <div className="mixer-rack-info">
              <strong>CHANNEL STRIP RACK</strong>
              <small>{tracks.length} Tracks ({audioTracks.length} Audio, {videoTracks.length} Video)</small>
            </div>
          </div>

          <div className="mixer-strips-scroll-area">
            <div className="fairlight-strips-grid">
              {tracks.map((track) => {
                const trackVu = vuLevels[track.id] ?? { left: 0, right: 0, peakDb: -60 };
                const currentGain = gainDrafts[track.id] ?? track.gain_db;
                const currentPan = panDrafts[track.id] ?? track.pan ?? 0;
                return (
                  <article
                    className={`fairlight-channel-strip ${track.id === activeTrackId ? "is-selected" : ""} ${track.muted ? "is-muted" : ""} ${track.solo ? "is-solo" : ""} ${track.locked ? "is-locked" : ""}`}
                    key={track.id}
                    onClick={() => setSelectedFairlightTrackId(track.id)}
                  >
                    <div className="strip-top">
                      <div className="strip-track-badge">
                        <span className={`strip-kind-pill ${track.kind}`}>
                          {track.kind === "audio" ? "A" : "V"}
                        </span>
                        <strong className="strip-track-name" title={track.name}>
                          {track.name}
                        </strong>
                      </div>
                      <div className="strip-meta">
                        <span className="strip-clip-count">
                          {track.clips.length} clip{track.clips.length !== 1 ? "s" : ""}
                        </span>
                      </div>
                    </div>

                    {/* Quick DSP Badges Row */}
                    <div className="strip-dsp-badges">
                      <InteriorButton
                        type="button"
                        className={`strip-dsp-btn ${track.audio_processing?.eq_enabled ? "active" : ""}`}
                        title={`EQ: ${track.audio_processing?.eq_enabled ? "ON" : "OFF"}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          void saveTrackAudioProcessing(track.id, { eq_enabled: !track.audio_processing?.eq_enabled });
                        }}
                      >
                        EQ
                      </InteriorButton>
                      <InteriorButton
                        type="button"
                        className={`strip-dsp-btn comp ${track.audio_processing?.comp_enabled ? "active" : ""}`}
                        title={`Dynamics: ${track.audio_processing?.comp_enabled ? "ON" : "OFF"}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          void saveTrackAudioProcessing(track.id, { comp_enabled: !track.audio_processing?.comp_enabled });
                        }}
                      >
                        DYN
                      </InteriorButton>
                      <InteriorButton
                        type="button"
                        className={`strip-dsp-btn denoise ${track.audio_processing?.denoise_enabled ? "active" : ""}`}
                        title={`Noise Reduction: ${track.audio_processing?.denoise_enabled ? "ON" : "OFF"}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          void saveTrackAudioProcessing(track.id, { denoise_enabled: !track.audio_processing?.denoise_enabled });
                        }}
                      >
                        NR
                      </InteriorButton>
                    </div>

                    {/* Track Bus Routing */}
                    <div className="strip-bus-routing" onClick={(e) => e.stopPropagation()}>
                      <span className="strip-bus-label">BUS ASSIGN</span>
                      <select
                        className="strip-bus-select"
                        value={track.bus ?? "master"}
                        onChange={(e) => void setTrackBusRoute(track.id, e.target.value as BusTarget)}
                      >
                        <option value="master">MASTER (Stereo Main)</option>
                        <option value="submix1">SUBMIX 1 (DX Dialogue)</option>
                        <option value="submix2">SUBMIX 2 (MX Music)</option>
                        <option value="aux_fx">AUX FX (Reverb / Echo)</option>
                      </select>
                    </div>

                    {/* Track Automation Mode Selector */}
                    <div className="strip-automation-section" onClick={(e) => e.stopPropagation()}>
                      <div className="strip-automation-pills">
                        {(["off", "read", "touch", "latch", "write"] as AutomationMode[]).map((mode) => (
                          <InteriorButton
                            key={mode}
                            type="button"
                            className={`auto-pill ${(track.automation?.mode ?? "off") === mode ? "active" : ""}`}
                            title={`Automation mode: ${mode.toUpperCase()}`}
                            onClick={() => void setTrackAutomationMode(track.id, mode)}
                          >
                            {mode.slice(0, 3).toUpperCase()}
                          </InteriorButton>
                        ))}
                      </div>
                    </div>

                    {/* Audio Plugins VST3 Slot with Stacked Header to Prevent Clipping */}
                    <div className="strip-plugins-rack" onClick={(e) => e.stopPropagation()}>
                      <div className="strip-plugins-header">
                        <span>PLUGINS</span>
                        <select
                          className="strip-add-plugin-select"
                          value=""
                          onChange={(e) => {
                            if (e.target.value) {
                              void addTrackPlugin(track.id, e.target.value);
                              e.target.value = "";
                            }
                          }}
                        >
                          <option value="" disabled>+ VST3…</option>
                          <option value="Saturn Dynamics">VST3: Saturn Dynamics</option>
                          <option value="Saturn Reverb">VST3: Studio Reverb</option>
                          <option value="Saturn De-Esser">VST3: De-Esser Pro</option>
                          <option value="Saturn Chorus">VST3: Chorus &amp; Widener</option>
                        </select>
                      </div>
                      {(track.plugins ?? []).map((plugin) => (
                        <div key={plugin.id} className={`plugin-rack-item ${plugin.enabled ? "enabled" : "bypassed"}`}>
                          <InteriorButton
                            type="button"
                            className={`plugin-power-dot ${plugin.enabled ? "active" : ""}`}
                            title={plugin.enabled ? "Bypass Plugin" : "Enable Plugin"}
                            onClick={() => void toggleTrackPlugin(track.id, plugin.id, !plugin.enabled)}
                          />
                          <span className="plugin-name-label" title={plugin.name}>{plugin.name}</span>
                          <span className="plugin-format-badge">VST3</span>
                        </div>
                      ))}
                    </div>

                    {/* Pan Section */}
                    <div className="strip-pan-section">
                      <div className="strip-pan-header">
                        <span className="strip-section-label">PAN</span>
                        <span className="strip-pan-val">{formatPan(currentPan)}</span>
                        <InteriorButton
                          type="button"
                          className="strip-pan-center-btn"
                          title="Reset pan to center"
                          onClick={() => void saveTrackPan(track.id, 0)}
                        >
                          C
                        </InteriorButton>
                      </div>
                      <input
                        type="range"
                        className="strip-pan-slider"
                        min="-1"
                        max="1"
                        step="0.02"
                        value={currentPan}
                        onChange={(e) => {
                          const val = Number(e.target.value);
                          setPanDrafts((prev) => ({ ...prev, [track.id]: val }));
                          audioClockMaster.updateTrackPan(track.id, val);
                        }}
                        onPointerUp={(e) => void saveTrackPan(track.id, Number(e.currentTarget.value))}
                        onKeyUp={(e) => void saveTrackPan(track.id, Number(e.currentTarget.value))}
                      />
                    </div>

                    {/* Matrix M / S / L Buttons */}
                    <div className="strip-matrix-buttons">
                      <InteriorButton
                        type="button"
                        className={`strip-btn btn-mute ${track.muted ? "active" : ""}`}
                        title={track.muted ? "Unmute track" : "Mute track"}
                        onClick={() => void toggleTrackMute(track)}
                      >
                        M
                      </InteriorButton>
                      <InteriorButton
                        type="button"
                        className={`strip-btn btn-solo ${track.solo ? "active" : ""}`}
                        title={track.solo ? "Solo track" : "Solo track"}
                        onClick={() => void toggleTrackSolo(track)}
                      >
                        S
                      </InteriorButton>
                      <InteriorButton
                        type="button"
                        className={`strip-btn btn-lock ${track.locked ? "active" : ""}`}
                        title={track.locked ? "Unlock track" : "Lock track"}
                        onClick={() => void toggleTrackLock(track)}
                      >
                        {track.locked ? "🔒" : "🔓"}
                      </InteriorButton>
                    </div>

                    {/* Meter + Vertical Fader Section */}
                    <div className="strip-meter-fader-row">
                      <div className="strip-vu-meter-column" title={`Peak: ${formatDb(trackVu.peakDb)}`}>
                        <div className="vu-meter-pair">
                          <div className="vu-bar-slot">
                            <div
                              className="vu-bar-fill left"
                              style={{ height: `${Math.min(100, (trackVu.left || 0) * 100)}%` }}
                            />
                          </div>
                          <div className="vu-bar-slot">
                            <div
                              className="vu-bar-fill right"
                              style={{ height: `${Math.min(100, (trackVu.right || 0) * 100)}%` }}
                            />
                          </div>
                          {track.audio_processing?.comp_enabled && (
                            <div className="strip-mini-gr-slot" title={`Gain Reduction: ${(trackGr[track.id] || 0).toFixed(1)} dB`}>
                              <div
                                className="strip-mini-gr-fill"
                                style={{ height: `${Math.min(100, Math.max(0, -(trackGr[track.id] || 0) * 4.16))}%` }}
                              />
                            </div>
                          )}
                        </div>
                        <div className="vu-db-readout">{formatDb(trackVu.peakDb)}</div>
                      </div>

                      <div className="strip-fader-column">
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
                            className="vertical-fader"
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
                          />
                        </div>
                      </div>
                    </div>

                    <div className="strip-fader-footer">
                      <InteriorButton
                        type="button"
                        className="strip-unity-btn"
                        title="Reset track gain to 0.0 dB unity"
                        onClick={() => void saveTrackGain(track.id, 0)}
                      >
                        0 dB
                      </InteriorButton>
                      <span className="strip-gain-text">
                        {currentGain > 0 ? "+" : ""}{currentGain.toFixed(1)} dB
                      </span>
                    </div>
                  </article>
                );
              })}
            </div>
          </div>
        </section>
  );
}
