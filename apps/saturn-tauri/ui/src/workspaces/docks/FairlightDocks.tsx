import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction } from "react";
import { formatDb } from "../../utils/formatters";
import type {
  Track,
  TrackAudioProcessing,
  BusTarget,
  AutomationMode,
} from "../../types";
import type { AudioClockMaster } from "../../engine/playbackEngine";
import { FairlightMixerRack } from "../FairlightMixerRack";
import { AutomationLane } from "../../features/audio/AutomationLane";
import { LoudnessPanel, type LoudnessReport } from "../fairlight/LoudnessPanel";
import { MasterBusPanel } from "../fairlight/MasterBusPanel";
import { DspRackPanel } from "../fairlight/DspRackPanel";
import { useAutomationWriters } from "../fairlight/useAutomationWriters";
import { DockCard, type DockTab } from "../../features/dock/DockCard";
import { MediaPanel, type MediaPanelProps } from "../../features/media-pool";

export type { LoudnessReport };

export interface FairlightDockProps {
  tracks: Track[];
  playhead: number;
  programPlaying: boolean;
  timelineZoom: number;
  timelineWidth: number;
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
  clearAllSolos: () => Promise<void> | void;
  muteAllTracks: () => Promise<void> | void;
  toggleTrackMute: (track: Track) => Promise<void> | void;
  toggleTrackSolo: (track: Track) => Promise<void> | void;
  toggleTrackLock: (track: Track) => Promise<void> | void;
  setTrackBusRoute: (trackId: number, bus: BusTarget) => Promise<void> | void;
  setTrackAutomationMode: (trackId: number, mode: AutomationMode) => Promise<void> | void;
  addTrackPlugin: (trackId: number, pluginName: string) => Promise<void> | void;
  toggleTrackPlugin: (trackId: number, pluginId: number, enabled: boolean) => Promise<void> | void;
  audioClockMaster: AudioClockMaster;
  perform: (command: string, payload: Record<string, unknown>) => Promise<unknown>;
  mediaPanelProps: MediaPanelProps;
}

const useActiveFairlightTrack = (
  tracks: Track[],
  selectedFairlightTrackId: number | null,
): { activeTrackId: number; selectedAutomationTrack: Track | undefined } => {
  const audioTracks = tracks.filter((t) => t.kind === "audio");
  const activeTrackId = selectedFairlightTrackId ?? audioTracks[0]?.id ?? tracks[0]?.id ?? 1;
  return { activeTrackId, selectedAutomationTrack: tracks.find((t) => t.id === activeTrackId) };
};

/**
 * Audio-mode dock tabs for the WorkspaceShell (plan v4 G5). The Fairlight
 * mixer, master-bus/loudness rack, DSP rack, and automation lane dock around
 * the persistent program monitor and timeline; the page's own transport,
 * picture-sync preview, and audio timeline tray are retired — the shell
 * monitor owns the picture and the persistent timeline owns the tracks.
 */
export function FairlightDockLeft({
  tracks,
  playhead,
  programPlaying,
  selectedFairlightTrackId,
  setSelectedFairlightTrackId,
  trackGr,
  vuLevels,
  gainDrafts,
  setGainDrafts,
  saveTrackGain,
  panDrafts,
  setPanDrafts,
  saveTrackPan,
  saveTrackAudioProcessing,
  setTrackBusRoute,
  setTrackAutomationMode,
  addTrackPlugin,
  toggleTrackPlugin,
  toggleTrackMute,
  toggleTrackSolo,
  toggleTrackLock,
  audioClockMaster,
  masterVu,
  masterGr,
  masterGainDraft,
  setMasterGainDraft,
  busGainDrafts,
  setBusGainDrafts,
  loudnessStandard,
  setLoudnessStandard,
  loudnessReport,
  autoTrimLoudness,
  resetLoudness,
  perform,
  clearAllSolos,
  muteAllTracks,
  mediaPanelProps,
}: FairlightDockProps) {
  const { activeTrackId } = useActiveFairlightTrack(tracks, selectedFairlightTrackId);
  const { writeTrackAutomation, commitTrackAutomation } = useAutomationWriters({
    perform,
    playing: programPlaying,
    playhead,
  });

  const tabs: DockTab[] = [
    {
      id: "fairlight-mixer",
      title: "Mixer",
      render: () => (
        <div className="fairlight-mixer-dock-body">
          <div className="audio-matrix-global-actions" style={{ display: "flex", gap: "4px", marginBottom: "8px" }}>
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
      ),
    },
    {
      id: "fairlight-master",
      title: "Master Bus",
      render: () => (
        <div className="fairlight-master-dock-body">
          <MasterBusPanel
            programPlaying={programPlaying}
            masterVu={masterVu}
            masterGr={masterGr}
            masterGainDraft={masterGainDraft}
            setMasterGainDraft={setMasterGainDraft}
            perform={perform}
          />
          <LoudnessPanel
            loudnessStandard={loudnessStandard}
            setLoudnessStandard={setLoudnessStandard}
            loudnessReport={loudnessReport}
            autoTrimLoudness={autoTrimLoudness}
            resetLoudness={resetLoudness}
            audioClockMaster={audioClockMaster}
          />
          <div
            className="audio-submix-buses-row"
            style={{ display: "flex", gap: "8px", marginTop: "10px", justifyContent: "center" }}
          >
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
                    onPointerUp={(e) =>
                      void perform("bus.set_gain_db", { target: b.target, gain_db: Number(e.currentTarget.value) })
                    }
                  />
                </div>
                <span style={{ fontSize: "10px", fontWeight: 700, color: "var(--text-primary)" }}>
                  {formatDb(busGainDrafts[b.target] ?? 0)}
                </span>
              </div>
            ))}
          </div>
        </div>
      ),
    },
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
  ];
  return <DockCard className="shell-dock shell-dock-left" tabs={tabs} ariaLabel="Fairlight dock" />;
}

export function FairlightDockRight({
  tracks,
  playhead,
  programPlaying,
  timelineZoom,
  timelineWidth,
  selectedFairlightTrackId,
  setSelectedFairlightTrackId,
  trackGr,
  audioProcessingDrafts,
  setAudioProcessingDrafts,
  saveTrackAudioProcessing,
  audioClockMaster,
  perform,
}: Pick<
  FairlightDockProps,
  | "tracks"
  | "playhead"
  | "programPlaying"
  | "timelineZoom"
  | "timelineWidth"
  | "selectedFairlightTrackId"
  | "setSelectedFairlightTrackId"
  | "trackGr"
  | "audioProcessingDrafts"
  | "setAudioProcessingDrafts"
  | "saveTrackAudioProcessing"
  | "audioClockMaster"
  | "perform"
>) {
  const { activeTrackId, selectedAutomationTrack } = useActiveFairlightTrack(tracks, selectedFairlightTrackId);
  const {
    addAutomationLanePoint,
    removeAutomationLanePoint,
    clearAutomationLane,
    setAutomationCurveInterpolation,
  } = useAutomationWriters({ perform, playing: programPlaying, playhead });

  const tabs: DockTab[] = [
    {
      id: "fairlight-dsp",
      title: "Track DSP",
      render: () => (
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
      ),
    },
    {
      id: "fairlight-automation",
      title: "Automation",
      render: () => (
        <div className="fairlight-automation-dock-body">
          {selectedAutomationTrack ? (
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
          ) : (
            <div className="effect-empty-card">
              <p>Select an audio track in the timeline to edit its automation lane.</p>
            </div>
          )}
        </div>
      ),
    },
  ];
  return <DockCard className="shell-dock shell-dock-right" tabs={tabs} ariaLabel="Fairlight DSP dock" />;
}
