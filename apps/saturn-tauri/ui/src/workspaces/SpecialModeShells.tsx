import type { ReactNode } from "react";
import type { Clip, MediaAsset, ProjectDocument } from "../app/projectTypes";
import type { AudioClockMaster } from "../engine/playbackEngine";
import type { MediaPanelProps } from "../features/media-pool";
import type { InspectorPanelProps } from "../features/inspector";
import type { MarkersPanelProps } from "../features/markers";
import type { CaptionsPanelProps } from "../features/captions";
import type { GraphicOperations } from "../features/graphics";
import { FairlightDockLeft, FairlightDockRight } from "./docks/FairlightDocks";
import { FusionDockRight } from "./docks/FusionDocks";
import { KeyframesDockRight } from "./docks/KeyframesDocks";
import { MediaDockLeft } from "./docks/MediaDocks";
import { MediaPoolDockLeft } from "./docks/MediaPoolDock";
import { EditDockRight } from "./docks/EditDocks";

export interface SpecialPanelProps {
  media: MediaPanelProps;
  inspector: InspectorPanelProps;
  markers: MarkersPanelProps;
  captions: CaptionsPanelProps;
  graphics: { ops: GraphicOperations; playhead: number; frame: { width: number; height: number } };
}

/**
 * Dock-tab composition for the docked special modes (media, fusion, audio,
 * keyframes) inside the persistent WorkspaceShell (plan v4 G5). The router
 * keeps returning the SAME WorkspaceShell element type for every docked mode
 * (React reconciliation must not remount the skeleton — geometry invariance),
 * so this module only picks the left/right dock nodes per mode; the long prop
 * lists live here to keep WorkspaceRouter under the 800-line gate.
 * Deliver is the user-directed exception: WorkspaceRouter routes it to the
 * full-page ExportWorkspace BEFORE the shell, so it never appears here.
 * NOTE: the Deliver page is intentionally left as its previous full-page
 * version — do not dock it, do not modify its layout.
 */
export interface SpecialModeDockArgs {
  davinciWorkspace: string;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  selectedMedia: MediaAsset | null;
  selectedTimelineClip: Clip | null;
  busy: boolean;
  missingMediaIds: Set<number>;
  timelineWidth: number;
  selectDavinciWorkspace: (ws: unknown) => void;
  playback: any;
  mediaOps: any;
  timelineOps: any;
  inspectorOps: any;
  projectOps: any;
  audioOps: any;
  audioClockMaster: AudioClockMaster;
  invokeMenuCommand: (id: string) => Promise<void>;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  panelProps: SpecialPanelProps;
}

export function specialModeDocks({
  davinciWorkspace,
  project,
  media,
  selectedTimelineClip,
  busy,
  missingMediaIds,
  timelineWidth,
  selectDavinciWorkspace,
  playback,
  mediaOps,
  timelineOps,
  inspectorOps,
  audioOps,
  audioClockMaster,
  perform,
  panelProps,
}: SpecialModeDockArgs): { left: ReactNode; right: ReactNode } {
  if (davinciWorkspace === "media") {
    return {
      left: (
        <MediaDockLeft
          project={project}
          media={media}
          visibleMedia={mediaOps.visibleMedia}
          selectedMediaId={mediaOps.selectedMediaId}
          setSelectedMediaId={mediaOps.setSelectedMediaId}
          setSelectedClip={timelineOps.setSelectedClip}
          setSelectedClips={timelineOps.setSelectedClips}
          selectDavinciWorkspace={selectDavinciWorkspace}
          importMedia={mediaOps.importMedia}
          setNetworkImportModalOpen={mediaOps.setNetworkImportModalOpen}
          search={mediaOps.search}
          setSearch={mediaOps.setSearch}
          busy={busy}
          missingMediaIds={missingMediaIds}
        />
      ),
      right: (
        <EditDockRight
          inspectorPanelProps={panelProps.inspector}
          markersPanelProps={panelProps.markers}
          captionsPanelProps={panelProps.captions}
          graphicsPanelProps={panelProps.graphics}
        />
      ),
    };
  }

  if (davinciWorkspace === "fusion") {
    return {
      left: <MediaPoolDockLeft mediaPanelProps={panelProps.media} ariaLabel="Fusion dock" />,
      right: (
        <FusionDockRight
          project={project}
          media={media}
          selectedNodeId={null}
          setSelectedNodeId={() => {}}
          connectingSocket={null}
          setConnectingSocket={() => {}}
          draggingNodeId={null}
          setDraggingNodeId={() => {}}
          nodeDragOffset={{ x: 0, y: 0 }}
          setNodeDragOffset={() => {}}
          updateFusionNodePos={async () => {}}
          connectFusionSockets={async () => {}}
          disconnectFusionSockets={async () => {}}
          addFusionNode={async () => {}}
          removeFusionNode={async () => {}}
          updateFusionNodeParams={async () => {}}
          fusionMaskFeather={8}
          setFusionMaskFeather={() => {}}
          fusionMaskInverted={false}
          setFusionMaskInverted={() => {}}
          fusionMaskPoints={[]}
          addRotoSplineMask={async () => {}}
          setNotice={() => {}}
        />
      ),
    };
  }

  if (davinciWorkspace === "audio") {
    const fairlightLeft = (
      <FairlightDockLeft
        tracks={project?.sequence.tracks ?? []}
        playhead={playback.playhead}
        programPlaying={playback.programPlaying}
        timelineZoom={timelineOps.timelineZoom}
        timelineWidth={timelineWidth}
        selectedFairlightTrackId={audioOps.selectedFairlightTrackId}
        setSelectedFairlightTrackId={audioOps.setSelectedFairlightTrackId}
        audioProcessingDrafts={audioOps.audioProcessingDrafts}
        setAudioProcessingDrafts={audioOps.setAudioProcessingDrafts}
        saveTrackAudioProcessing={audioOps.saveTrackAudioProcessing}
        trackGr={audioOps.trackGr}
        masterGr={audioOps.masterGr}
        vuLevels={audioOps.vuLevels}
        masterVu={audioOps.masterVu}
        gainDrafts={timelineOps.gainDrafts}
        setGainDrafts={timelineOps.setGainDrafts}
        saveTrackGain={timelineOps.saveTrackGain}
        panDrafts={timelineOps.panDrafts}
        setPanDrafts={timelineOps.setPanDrafts}
        saveTrackPan={timelineOps.saveTrackPan}
        masterGainDraft={audioOps.masterGainDraft}
        setMasterGainDraft={audioOps.setMasterGainDraft}
        busGainDrafts={audioOps.busGainDrafts}
        setBusGainDrafts={audioOps.setBusGainDrafts}
        loudnessStandard={audioOps.loudnessStandard}
        setLoudnessStandard={audioOps.setLoudnessStandard}
        loudnessReport={audioOps.loudnessReport}
        autoTrimLoudness={audioOps.autoTrimLoudness}
        resetLoudness={audioOps.resetLoudness}
        clearAllSolos={audioOps.clearAllSolos}
        muteAllTracks={audioOps.muteAllTracks}
        toggleTrackMute={timelineOps.toggleTrackMute}
        toggleTrackSolo={timelineOps.toggleTrackSolo}
        toggleTrackLock={timelineOps.toggleTrackLock}
        setTrackBusRoute={audioOps.setTrackBusRoute}
        setTrackAutomationMode={audioOps.setTrackAutomationMode}
        addTrackPlugin={audioOps.addTrackPlugin}
        toggleTrackPlugin={audioOps.toggleTrackPlugin}
        audioClockMaster={audioClockMaster}
        perform={async (cmd, payload) => perform(cmd, payload)}
        mediaPanelProps={panelProps.media}
      />
    );
    const fairlightRight = (
      <FairlightDockRight
        tracks={project?.sequence.tracks ?? []}
        playhead={playback.playhead}
        programPlaying={playback.programPlaying}
        timelineZoom={timelineOps.timelineZoom}
        timelineWidth={timelineWidth}
        selectedFairlightTrackId={audioOps.selectedFairlightTrackId}
        setSelectedFairlightTrackId={audioOps.setSelectedFairlightTrackId}
        audioProcessingDrafts={audioOps.audioProcessingDrafts}
        setAudioProcessingDrafts={audioOps.setAudioProcessingDrafts}
        saveTrackAudioProcessing={audioOps.saveTrackAudioProcessing}
        trackGr={audioOps.trackGr}
        audioClockMaster={audioClockMaster}
        perform={async (cmd, payload) => perform(cmd, payload)}
      />
    );
    return { left: fairlightLeft, right: fairlightRight };
  }

  // keyframes
  return {
    left: <MediaPoolDockLeft mediaPanelProps={panelProps.media} ariaLabel="Keyframes dock" />,
    right: (
      <KeyframesDockRight
        media={media}
        selectedTimelineClip={selectedTimelineClip}
        keyframeProperty={inspectorOps.keyframeProperty}
        setKeyframeProperty={inspectorOps.setKeyframeProperty}
        keyframeValue={inspectorOps.keyframeValue}
        setKeyframeValue={inspectorOps.setKeyframeValue}
        keyframeInterpDraft={inspectorOps.keyframeInterpDraft}
        setKeyframeInterpDraft={inspectorOps.setKeyframeInterpDraft}
        keyframeHandleInX={inspectorOps.keyframeHandleInX}
        setKeyframeHandleInX={() => {}}
        keyframeHandleInY={inspectorOps.keyframeHandleInY}
        setKeyframeHandleInY={() => {}}
        keyframeHandleOutX={inspectorOps.keyframeHandleOutX}
        setKeyframeHandleOutX={() => {}}
        keyframeHandleOutY={inspectorOps.keyframeHandleOutY}
        setKeyframeHandleOutY={() => {}}
        selectedKeyframeTime={inspectorOps.selectedKeyframeTime}
        setSelectedKeyframeTime={inspectorOps.setSelectedKeyframeTime}
        playhead={playback.playhead}
        seekProgramTo={playback.seekProgramTo}
        programPlaying={playback.programPlaying}
        toggleProgramPlayback={playback.toggleProgramPlayback}
        saveKeyframe={inspectorOps.saveKeyframe}
        deleteKeyframe={inspectorOps.deleteKeyframe}
      />
    ),
  };
}
