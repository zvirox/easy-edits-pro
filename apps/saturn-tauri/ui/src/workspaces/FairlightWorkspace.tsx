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
import { AutomationLane } from "../features/audio/AutomationLane";
import { LoudnessPanel, type LoudnessReport } from "./fairlight/LoudnessPanel";
import { MasterBusPanel } from "./fairlight/MasterBusPanel";
import { DspRackPanel } from "./fairlight/DspRackPanel";
import { useAutomationWriters } from "./fairlight/useAutomationWriters";

export type { LoudnessReport };

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
  const selectedAutomationTrack = tracks.find((t) => t.id === activeTrackId);

  const {
    writeTrackAutomation,
    commitTrackAutomation,
    addAutomationLanePoint,
    removeAutomationLanePoint,
    clearAutomationLane,
    setAutomationCurveInterpolation,
  } = useAutomationWriters({ perform, playing: programPlaying, playhead });

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

          <MasterBusPanel
            programPlaying={programPlaying}
            masterVu={masterVu}
            masterGr={masterGr}
            masterGainDraft={masterGainDraft}
            setMasterGainDraft={setMasterGainDraft}
            perform={perform}
          />

          {/* EBU R128 Broadcast Loudness Meter & Auto-Trim */}
          <LoudnessPanel
            loudnessStandard={loudnessStandard}
            setLoudnessStandard={setLoudnessStandard}
            loudnessReport={loudnessReport}
            autoTrimLoudness={autoTrimLoudness}
            resetLoudness={resetLoudness}
            audioClockMaster={audioClockMaster}
          />

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
        <DspRackPanel
          tracks={tracks}
          activeTrackId={activeTrackId}
          trackGr={trackGr}
          audioProcessingDrafts={audioProcessingDrafts}
          setAudioProcessingDrafts={setAudioProcessingDrafts}
          saveTrackAudioProcessing={saveTrackAudioProcessing}
          audioClockMaster={audioClockMaster}
          setSelectedFairlightTrackId={setSelectedFairlightTrackId}
        />

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
          writeTrackAutomation={writeTrackAutomation}
          commitTrackAutomation={commitTrackAutomation}
        />
      </div>

      {/* Automation lane editor for the selected track */}
      {selectedAutomationTrack && (
        <AutomationLane
          track={selectedAutomationTrack}
          playhead={playhead}
          timelineZoom={timelineZoom}
          timelineWidth={timelineWidth}
          lane="volume"
          onChangeLane={() => {}}
          addPoint={addAutomationLanePoint}
          removePoint={removeAutomationLanePoint}
          clearLane={clearAutomationLane}
          setInterpolation={setAutomationCurveInterpolation}
          audioClock={audioClockMaster}
        />
      )}

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
