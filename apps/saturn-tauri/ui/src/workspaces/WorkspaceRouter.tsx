import { MediaWorkspace } from "./MediaWorkspace";
import type { MediaPanelProps } from "../features/media-pool";
import type { InspectorPanelProps } from "../features/inspector";
import type { MarkersPanelProps } from "../features/markers";
import type { CaptionsPanelProps } from "../features/captions";
import { ExportWorkspace } from "./ExportWorkspace";
import { FairlightWorkspace } from "./FairlightWorkspace";
import { FusionWorkspace } from "./FusionWorkspace";
import { KeyframesWorkspace } from "./KeyframesWorkspace";
import { WorkspaceShell } from "./WorkspaceShell";
import {
  EditDockLeft,
  EditDockRight,
  type TrimMonitorEntryProps,
} from "./docks/EditDocks";
import { ColorDockLeft, ColorDockRight } from "./docks/ColorDocks";
import { EffectDockLeft, EffectDockRight } from "./docks/EffectsDocks";
import { PreviewSection, type PreviewSectionProps } from "../features/preview";
import {
  TimelineSection,
  TimelineToolRail,
  type TimelineSectionProps,
} from "../features/timeline";
import { useEffect, useRef, useState } from "react";
import { TICKS_PER_SECOND, type Clip, type MediaAsset, type ProjectDocument } from "../app/projectTypes";
import type { CommandStatus } from "../app/ProjectProvider";
import type { AudioClockMaster } from "../engine/playbackEngine";
import type { SourcePreviewFocus } from "../features/preview";
import type { CommandSpec } from "../features/menubar/AppHeader";
import type { MarkerOperations } from "../app/useMarkers";
import type { CaptionOperations } from "../app/useCaptions";
import type { GraphicOperations } from "../features/graphics";

export interface WorkspaceRouterProps {
  commands: CommandSpec[];
  davinciWorkspace: string;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  getMediaUrl: (path: string) => string;
  selectedTimelineClip: Clip | null;
  selectedMedia: MediaAsset | null;
  missingMediaIds: Set<number>;
  busy: boolean;
  timelineWidth: number;
  timelineEndSeconds: number;
  selectDavinciWorkspace: (ws: any) => void;
  playback: any;
  mediaOps: any;
  timelineOps: any;
  inspectorOps: any;
  projectOps: any;
  invokeMenuCommand: (id: string) => Promise<void>;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  programPreviewError: string | null;
  audioClockMaster: any;
  audioOps: any;
  markerOps: MarkerOperations;
  captionsOps: CaptionOperations;
  graphicsOps: GraphicOperations;
}

export function WorkspaceRouter({
  commands,
  davinciWorkspace,
  project,
  media,
  getMediaUrl,
  selectedTimelineClip,
  selectedMedia,
  missingMediaIds,
  busy,
  timelineWidth,
  timelineEndSeconds,
  selectDavinciWorkspace,
  playback,
  mediaOps,
  timelineOps,
  inspectorOps,
  projectOps,
  invokeMenuCommand,
  perform,
  undoStatus,
  redoStatus,
  programPreviewError,
  audioClockMaster,
  audioOps,
  markerOps,
  captionsOps,
  graphicsOps,
}: WorkspaceRouterProps) {
  // Unified monitor Source-tab focus: a click (pointer down→up without drag)
  // on a media-pool item or a timeline clip loads it in the Source tab.
  // Automated selections — media drops, add-to-timeline — must not steal
  // the monitor, so the switch is driven by gestures, not selection state.
  const [sourcePreview, setSourcePreview] = useState<SourcePreviewFocus | null>(null);
  const mediaRef = useRef(media);
  useEffect(() => {
    mediaRef.current = media;
  }, [media]);

  useEffect(() => {
    const gesture = { kind: "none" as "none" | "clip" | "media", startX: 0, startY: 0, moved: false };

    const onPointerDown = (event: globalThis.PointerEvent) => {
      const target = event.target as Element | null;
      gesture.kind = target?.closest?.(".timeline-clip[data-media-id]")
        ? "clip"
        : target?.closest?.(".media-item[data-media-id]")
          ? "media"
          : "none";
      gesture.startX = event.clientX;
      gesture.startY = event.clientY;
      gesture.moved = false;
    };

    const onPointerMove = (event: globalThis.PointerEvent) => {
      if (gesture.kind === "none") return;
      if (Math.abs(event.clientX - gesture.startX) > 6 || Math.abs(event.clientY - gesture.startY) > 6) {
        gesture.moved = true;
      }
    };

    const onPointerUp = (event: globalThis.PointerEvent) => {
      if (gesture.kind === "none" || gesture.moved) return;
      const target = event.target as Element | null;
      if (gesture.kind === "clip") {
        const clipEl = target?.closest?.(".timeline-clip[data-media-id]") as HTMLElement | null;
        if (!clipEl) return;
        const mediaId = Number(clipEl.dataset.mediaId);
        const asset = mediaRef.current.find((item) => item.id === mediaId);
        if (!asset) return;
        const sourceInTicks = Number(clipEl.dataset.sourceIn ?? "0");
        setSourcePreview({
          asset,
          startSeconds: sourceInTicks / TICKS_PER_SECOND,
          key: `clip-${mediaId}-${clipEl.dataset.timelineStart}-${sourceInTicks}`,
        });
      } else {
        const itemEl = target?.closest?.(".media-item[data-media-id]") as HTMLElement | null;
        if (!itemEl) return;
        const mediaId = Number(itemEl.dataset.mediaId);
        const asset = mediaRef.current.find((item) => item.id === mediaId);
        if (!asset) return;
        setSourcePreview({ asset, startSeconds: 0, key: `media-${asset.id}-${asset.name}` });
      }
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("pointermove", onPointerMove, true);
    document.addEventListener("pointerup", onPointerUp, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("pointermove", onPointerMove, true);
      document.removeEventListener("pointerup", onPointerUp, true);
    };
  }, []);

  if (davinciWorkspace === "media") {
    return (
      <MediaWorkspace
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
    );
  }

  if (davinciWorkspace === "deliver") {
    return (
      <ExportWorkspace
        project={project}
        batchQueue={[]}
        setBatchQueue={() => {}}
        queuePaused={false}
        toggleQueuePause={() => {}}
        deliverDestination="render"
        setDeliverDestination={() => {}}
        handleExportTimelineInterop={projectOps.handleExportTimelineInterop}
        handleImportTimelineInterop={projectOps.handleImportTimelineInterop}
        isInteropBusy={projectOps.isInteropBusy}
        interopContent={projectOps.interopContent}
        setNotice={projectOps.setNotice}
        encoderInfo={null}
        selectedExportPreset="source"
        setSelectedExportPreset={() => {}}
        exportResolution={projectOps.exportResolution}
        setExportResolution={projectOps.setExportResolution}
        exportFrameRate={projectOps.exportFrameRate}
        setExportFrameRate={projectOps.setExportFrameRate}
        exportFormat={projectOps.exportFormat}
        setExportFormat={projectOps.setExportFormat}
        exportFileName={projectOps.exportFileName}
        setExportFileName={projectOps.setExportFileName}
        exportOutputPath={projectOps.exportOutputPath}
        setExportOutputPath={projectOps.setExportOutputPath}
        chooseExportDestination={projectOps.chooseExportDestination}
        useHardwareEncoding={projectOps.useHardwareEncoding}
        setUseHardwareEncoding={projectOps.setUseHardwareEncoding}
        exportMarkedRange={projectOps.exportMarkedRange}
        setExportMarkedRange={projectOps.setExportMarkedRange}
        invokeMenuCommand={invokeMenuCommand}
        activeClip={null}
        selectedMedia={selectedMedia}
        renderPreview={() => null}
        rendering={projectOps.rendering}
        renderProgress={projectOps.renderProgress}
        addExportToQueue={() => {}}
        exportProject={projectOps.exportProject}
      />
    );
  }

  if (davinciWorkspace === "fusion") {
    return (
      <FusionWorkspace
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
    );
  }

  if (davinciWorkspace === "audio") {
    return (
      <FairlightWorkspace
        project={project}
        media={media}
        playhead={playback.playhead}
        timelineZoom={timelineOps.timelineZoom}
        timelineWidth={timelineWidth}
        selectedClip={timelineOps.selectedClip}
        setSelectedClip={timelineOps.setSelectedClip}
        setSelectedClips={timelineOps.setSelectedClips}
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
        programPlaying={playback.programPlaying}
        toggleProgramPlayback={playback.toggleProgramPlayback}
        stepProgramFrame={playback.stepProgramFrame}
        handleShuttleKey={playback.handleShuttleKey}
        clearAllSolos={audioOps.clearAllSolos}
        muteAllTracks={audioOps.muteAllTracks}
        toggleTrackMute={timelineOps.toggleTrackMute}
        toggleTrackSolo={timelineOps.toggleTrackSolo}
        toggleTrackLock={timelineOps.toggleTrackLock}
        setTrackBusRoute={audioOps.setTrackBusRoute}
        setTrackAutomationMode={audioOps.setTrackAutomationMode}
        addTrackPlugin={audioOps.addTrackPlugin}
        toggleTrackPlugin={audioOps.toggleTrackPlugin}
        zoomTimeline={timelineOps.zoomTimeline}
        beginTimelineScrub={timelineOps.beginTimelineScrub}
        moveTimelineScrub={timelineOps.moveTimelineScrub}
        finishTimelineScrub={timelineOps.finishTimelineScrub}
        renderProgramMonitor={() => null}
        audioClockMaster={audioClockMaster}
        perform={async (cmd, payload) => perform(cmd, payload)}
      />
    );
  }

  const mediaPanelProps: MediaPanelProps = {
        filmoraTab: mediaOps.filmoraTab,
        setFilmoraTab: mediaOps.setFilmoraTab,
        search: mediaOps.search,
        setSearch: mediaOps.setSearch,
        project,
        media,
        visibleMedia: mediaOps.visibleMedia,
        selectedMediaId: mediaOps.selectedMediaId,
        setSelectedMediaId: (id: number) => mediaOps.setSelectedMediaId(id),
        missingMediaIds,
        importMedia: mediaOps.importMedia,
        setNetworkImportModalOpen: mediaOps.setNetworkImportModalOpen,
        setCreateBinModalOpen: mediaOps.setCreateBinModalOpen,
        setMediaEditPropsId: mediaOps.setMediaEditPropsId,
        addPresetTitle: mediaOps.addPresetTitle,
        addPresetTransition: mediaOps.addPresetTransition,
        assetQuery: mediaOps.assetQuery,
        setAssetQuery: mediaOps.setAssetQuery,
        assetType: mediaOps.assetType,
        setAssetType: mediaOps.setAssetType,
        busy,
        searchAssets: mediaOps.searchAssets,
        assetResults: mediaOps.assetResults,
        addLibraryAsset: mediaOps.addLibraryAsset,
        setTranscribeModalOpen: projectOps.setTranscribeModalOpen,
        handleDetectSilence: projectOps.handleDetectSilence,
        setSilenceModalOpen: projectOps.setSilenceModalOpen,
        aiModel: mediaOps.aiModel,
        setAiModel: mediaOps.setAiModel,
        aiPrompt: mediaOps.aiPrompt,
        setAiPrompt: mediaOps.setAiPrompt,
        askAi: mediaOps.askAi,
        aiReply: mediaOps.aiReply,
        checkMissingMedia: mediaOps.checkMissingMedia,
        mediaViewMode: mediaOps.mediaViewMode,
        setMediaViewMode: mediaOps.setMediaViewMode,
        activeBinId: mediaOps.activeBinId,
        setActiveBinId: mediaOps.setActiveBinId,
        renameBin: mediaOps.renameMediaBin,
        deleteBin: mediaOps.deleteMediaBin,
        beginMediaPointer: mediaOps.beginMediaPointer,
        moveMediaPointer: mediaOps.moveMediaPointer,
        cancelMediaPointer: mediaOps.cancelMediaPointer,
        finishMediaPointer: mediaOps.finishMediaPointer,
        playhead: playback.playhead,
  };  const previewSectionProps: PreviewSectionProps = {
        selectedMedia,
        sourcePreview,
        getMediaUrl,
        sourcePreviewError: null,
        setSourcePreviewError: () => {},
        preparedPreviews: {},
        preparingPreviews: new Set(),
        previewPreparationErrors: {},
        mediaServerPort: 0,
        deckSnapshot: playback.deckSnapshot,
        deckARef: playback.deckARef,
        deckBRef: playback.deckBRef,
        programPreviewError,
        setProgramPreviewError: () => {},
        showTrackingBox: inspectorOps.showTrackingBox,
        audioPlayersRef: playback.audioPlayersRef,
        programPlaying: playback.programPlaying,
        toggleProgramPlayback: playback.toggleProgramPlayback,
        stepProgramFrame: playback.stepProgramFrame,
        shuttleSpeed: playback.shuttleSpeed,
        playhead: playback.playhead,
        project,
        media,
        multicamMode: false,
        setMulticamMode: () => {},
        multicamActiveAngle: 1,
        setMulticamActiveAngle: () => {},
        switchMulticamAngle: async () => {},
        syncMulticamWaveforms: async () => {},
        onThreePointEdit: (mode, mediaId, sourceIn, duration) =>
          timelineOps.threePointEdit(
            mode,
            mediaId,
            sourceIn,
            duration,
            playback.playhead,
          ),
        graphicsEditing: {
          activeTool: timelineOps.timelineTool,
          selectedId: graphicsOps.selectedId,
          onSelect: graphicsOps.setSelectedId,
          onCreate: (kind, xNorm, yNorm, vertical) =>
            graphicsOps.createAt(kind, xNorm, yNorm, vertical),
          onTransform: (id, patch) => graphicsOps.updateTransform(id, patch),
        },
  };  const timelineSectionProps: TimelineSectionProps = {
        timelineTool: timelineOps.timelineTool,
        setTimelineTool: timelineOps.setTimelineTool,
        project,
        media,
        switchSequence: timelineOps.switchSequence,
        playhead: playback.playhead,
        busy,
        addTrackToTimeline: timelineOps.addTrackToTimeline,
        timelineTrackHeight: timelineOps.timelineTrackHeight,
        setTimelineTrackHeight: timelineOps.setTimelineTrackHeight,
        snapEnabled: timelineOps.snapEnabled,
        setSnapEnabled: timelineOps.setSnapEnabled,
        addSelectedToTimeline: mediaOps.addSelectedToTimeline,
        selectedMedia,
        addTitleToTimeline: mediaOps.addTitleToTimeline,
        addTransitionToTimeline: mediaOps.addTransitionToTimeline,
        undoStatus,
        redoStatus,
        perform,
        commands,
        selectedClip: timelineOps.selectedClip,
        splitSelectedClip: timelineOps.splitSelectedClip,
        rippleDeleteSelectedClip: timelineOps.rippleDeleteSelectedClip,
        removeSelectedClip: timelineOps.removeSelectedClip,
        selectedClips: timelineOps.selectedClips,
        setSelectedClips: timelineOps.setSelectedClips,
        linkSelectedClips: timelineOps.linkSelectedClips,
        unlinkSelectedClips: timelineOps.unlinkSelectedClips,
        setCompoundClipNameDraft: projectOps.setCompoundClipNameDraft,
        setCompoundClipModalOpen: projectOps.setCompoundClipModalOpen,
        decomposeSelectedCompoundClip: timelineOps.decomposeSelectedCompoundClip,
        invokeMenuCommand,
        zoomTimeline: timelineOps.zoomTimeline,
        handleCreateTrackGroup: timelineOps.handleCreateTrackGroup,
        attachTimelineScroller: timelineOps.attachTimelineScroller,
        handleTimelinePanStart: timelineOps.handleTimelinePanStart,
        handleTimelinePanMove: timelineOps.handleTimelinePanMove,
        handleTimelinePanEnd: timelineOps.handleTimelinePanEnd,
        timelineWidth,
        timelineZoom: timelineOps.timelineZoom,
        beginTimelineScrub: timelineOps.beginTimelineScrub,
        moveTimelineScrub: timelineOps.moveTimelineScrub,
        finishTimelineScrub: timelineOps.finishTimelineScrub,
        timelineEndSeconds,
        addSubtitleAtPlayhead: inspectorOps.addSubtitleAtPlayhead,
        importSubtitlesDialog: inspectorOps.importSubtitlesDialog,
        exportSubtitlesDialog: inspectorOps.exportSubtitlesDialog,
        selectedSubtitle: inspectorOps.selectedSubtitle,
        setSelectedSubtitle: inspectorOps.setSelectedSubtitle,
        selectedMarkerId: markerOps.selectedMarkerId,
        onSelectMarker: markerOps.selectMarker,
        onOpenMarkerPanel: markerOps.openPanel,
        setSelectedClipId: timelineOps.setSelectedClipId,
        setSelectedClip: timelineOps.setSelectedClip,
        handleToggleGroupCollapse: timelineOps.handleToggleGroupCollapse,
        handleEditTrackGroup: timelineOps.handleEditTrackGroup,
        toggleTrackLock: timelineOps.toggleTrackLock,
        toggleTrackMute: timelineOps.toggleTrackMute,
        toggleTrackSolo: timelineOps.toggleTrackSolo,
        audioClockMaster,
        removeTrackFromTimeline: timelineOps.removeTrackFromTimeline,
        mediaDropTargetTrackId: timelineOps.mediaDropTargetTrackId,
        clipDragOffset: timelineOps.clipDragOffset,
        rippleCloseGap: timelineOps.rippleCloseGap,
        selectedTransition: timelineOps.selectedTransition,
        setSelectedTransition: timelineOps.setSelectedTransition,
        missingMediaIds,
        beginTimelineClipPointer: timelineOps.beginTimelineClipPointer,
        moveTimelineClipPointer: timelineOps.moveTimelineClipPointer,
        handleTimelineClipPointerCancel: timelineOps.handleTimelineClipPointerCancel,
        beginTimelineClipTrim: timelineOps.beginTimelineClipTrim,
        moveTimelineClipTrim: timelineOps.moveTimelineClipTrim,
        finishTimelineClipTrim: timelineOps.finishTimelineClipTrim,
        // New props for enhanced timeline
        sequences: project?.sequences,
        currentSequenceId: project?.sequence.id,
        addVideoTrack: () => timelineOps.addTrackToTimeline("video"),
        addAudioTrack: () => timelineOps.addTrackToTimeline("audio"),
        onMediaDropToEmptySpace: async (_x: number, _y: number, mediaId: number) => {
          // Determine track type from media
          const mediaItem = project?.media.find((m) => m.id === mediaId);
          if (!mediaItem) return;
          const trackKind = mediaItem.kind === "audio" ? "audio" : "video";
          await timelineOps.addTrackToTimeline(trackKind);
          // After adding track, we need to refresh to get the new track ID
          // The actual drop handling will be done by the track lane
        },
  };  const inspectorPanelProps: InspectorPanelProps = {
        workspace: projectOps.workspace,
        selectedSubtitle: inspectorOps.selectedSubtitle,
        setSelectedSubtitle: inspectorOps.setSelectedSubtitle,
        subtitleTextDraft: inspectorOps.subtitleTextDraft,
        setSubtitleTextDraft: inspectorOps.setSubtitleTextDraft,
        updateSubtitle: inspectorOps.updateSubtitle,
        addSubtitleAtPlayhead: inspectorOps.addSubtitleAtPlayhead,
        removeSubtitle: inspectorOps.removeSubtitle,
        clearSubtitles: inspectorOps.clearSubtitles,
        importSubtitlesDialog: inspectorOps.importSubtitlesDialog,
        exportSubtitlesDialog: inspectorOps.exportSubtitlesDialog,
        setTranscribeModalOpen: projectOps.setTranscribeModalOpen,
        setSilenceModalOpen: projectOps.setSilenceModalOpen,
        handleDetectSilence: projectOps.handleDetectSilence,
        selectedTimelineClip,
        selectedClip: timelineOps.selectedClip,
        colorDraft: inspectorOps.colorDraft,
        setColorDraft: inspectorOps.setColorDraft,
        saveColor: inspectorOps.saveColor,
        addEffect: inspectorOps.addEffect,
        toggleEffectEnabled: (id) => inspectorOps.toggleEffect(id),
        updateEffectParam: inspectorOps.updateEffectParam,
        saveEffectParam: inspectorOps.saveEffectParam,
        removeEffect: (id) => inspectorOps.removeEffect(id),
        playhead: playback.playhead,
        keyframeProperty: inspectorOps.keyframeProperty,
        setKeyframeProperty: inspectorOps.setKeyframeProperty,
        keyframeValue: inspectorOps.keyframeValue,
        setKeyframeValue: inspectorOps.setKeyframeValue,
        keyframeInterpDraft: inspectorOps.keyframeInterpDraft,
        setKeyframeInterpDraft: inspectorOps.setKeyframeInterpDraft,
        keyframeHandleInX: inspectorOps.keyframeHandleInX,
        keyframeHandleInY: inspectorOps.keyframeHandleInY,
        keyframeHandleOutX: inspectorOps.keyframeHandleOutX,
        keyframeHandleOutY: inspectorOps.keyframeHandleOutY,
        selectedKeyframeTime: inspectorOps.selectedKeyframeTime,
        setSelectedKeyframeTime: inspectorOps.setSelectedKeyframeTime,
        selectDavinciWorkspace,
        saveKeyframe: inspectorOps.saveKeyframe,
        deleteKeyframe: inspectorOps.deleteKeyframe,
        project,
        exportMarkedRange: projectOps.exportMarkedRange,
        setExportMarkedRange: projectOps.setExportMarkedRange,
        invokeMenuCommand,
        exportFormat: projectOps.exportFormat,
        setExportFormat: projectOps.setExportFormat,
        rendering: projectOps.rendering,
        exportProject: projectOps.exportProject,
        renderProgress: projectOps.renderProgress,
        assetQuery: mediaOps.assetQuery,
        setAssetQuery: mediaOps.setAssetQuery,
        assetType: mediaOps.assetType,
        setAssetType: mediaOps.setAssetType,
        busy,
        searchAssets: mediaOps.searchAssets,
        assetResults: mediaOps.assetResults,
        addLibraryAsset: mediaOps.addLibraryAsset,
        aiModel: mediaOps.aiModel,
        setAiModel: mediaOps.setAiModel,
        aiPrompt: mediaOps.aiPrompt,
        setAiPrompt: mediaOps.setAiPrompt,
        askAi: mediaOps.askAi,
        aiReply: mediaOps.aiReply,
        selectedTransition: timelineOps.selectedTransition,
        setSelectedTransition: timelineOps.setSelectedTransition,
        updateTransition: mediaOps.updateTransition,
        removeSelectedTransition: mediaOps.removeSelectedTransition,
        media,
        titleDraft: mediaOps.titleDraft,
        setTitleDraft: mediaOps.setTitleDraft,
        saveTitleProperty: mediaOps.saveTitleProperty,
        removeSelectedClip: timelineOps.removeSelectedClip,
        selectedMedia,
        unlinkSelectedClips: timelineOps.unlinkSelectedClips,
        trackingMode: inspectorOps.trackingMode,
        setTrackingMode: inspectorOps.setTrackingMode,
        showTrackingBox: inspectorOps.showTrackingBox,
        setShowTrackingBox: inspectorOps.setShowTrackingBox,
        trackingSmoothness: inspectorOps.trackingSmoothness,
        updateStabilizationSmoothness: inspectorOps.updateStabilizationSmoothness,
        trackingTargetClipId: inspectorOps.trackingTargetClipId,
        pinClipToMotionTrack: inspectorOps.pinClipToMotionTrack,
        isAnalyzingTracking: inspectorOps.isAnalyzingTracking,
        runMotionTracker: inspectorOps.runMotionTracker,
        saveSpeedRamp: inspectorOps.saveSpeedRamp,
        saveMotionBlur: inspectorOps.saveMotionBlur,
        masterVu: { left: -60, right: -60, peakDb: -60 },
        vuLevels: {},
        loudnessReport: undefined,
        masterGainDraft: 0,
        setMasterGainDraft: () => {},
        gainDrafts: timelineOps.gainDrafts,
        setGainDrafts: timelineOps.setGainDrafts,
        saveTrackGain: timelineOps.saveTrackGain,
        toggleTrackMute: timelineOps.toggleTrackMute,
        toggleTrackSolo: timelineOps.toggleTrackSolo,
        audioClockMaster: audioClockMaster as AudioClockMaster,
        activeClip: null,
  };  const trimMonitorProps: TrimMonitorEntryProps = {
        session: timelineOps.trimSession,
        project,
        media,
        getMediaUrl,
        onCommit: () => void timelineOps.commitTrimMode(),
        onCancel: timelineOps.cancelTrimMode,
  };  const markersPanelProps: MarkersPanelProps = {
        markers: markerOps.markers,
        selectedMarkerId: markerOps.selectedMarkerId,
        frameRate: project?.settings.frame_rate ?? { numerator: 30, denominator: 1 },
        onSelect: markerOps.setSelectedMarkerId,
        onSeek: (tick: number) => playback.seekProgramTo(tick),
        onRename: markerOps.renameMarker,
        onUpdateKind: markerOps.setMarkerKind,
        onDelete: markerOps.removeMarker,
        onClear: markerOps.clearMarkers,
        onAdd: markerOps.addMarker,
        onAddRange: markerOps.addRangeMarker,
        onImport: markerOps.importMarkers,
        onExport: markerOps.exportMarkers,
  };  const captionsPanelProps: CaptionsPanelProps = {
        track: captionsOps.track,
        cues: captionsOps.cues,
        playhead: playback.playhead,
        frameRate: project?.settings.frame_rate ?? { numerator: 30, denominator: 1 },
        selectedCueId: captionsOps.selectedCueId,
        onSelect: captionsOps.selectCue,
        onAdd: captionsOps.addCue,
        onEditText: captionsOps.updateCueText,
        onEditSpeaker: captionsOps.updateCueSpeaker,
        onNudge: captionsOps.nudgeCue,
        onDelete: captionsOps.removeCue,
        onImport: captionsOps.importCues,
        onExport: captionsOps.exportCues,
  };  const graphicsPanelProps: { ops: GraphicOperations; playhead: number; frame: { width: number; height: number } } = {
        ops: graphicsOps,
        playhead: playback.playhead,
        frame: project?.settings ?? { width: 1920, height: 1080 },
  };  if (davinciWorkspace === "keyframes") {
    return (
      <KeyframesWorkspace
        project={project}
        media={media}
        selectedClip={timelineOps.selectedClip}
        selectedTimelineClip={selectedTimelineClip}
        setSelectedClip={timelineOps.setSelectedClip}
        setSelectedClips={timelineOps.setSelectedClips}
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
    );
  }

  const shellToolbox = (
    <TimelineToolRail
      timelineTool={timelineSectionProps.timelineTool}
      setTimelineTool={timelineSectionProps.setTimelineTool}
    />
  );
  const shellTimeline = <TimelineSection {...timelineSectionProps} />;
  const shellCenter = <PreviewSection {...previewSectionProps} />;

  if (davinciWorkspace === "color" || davinciWorkspace === "effects") {
    const isColor = davinciWorkspace === "color";
    return (
      <WorkspaceShell
        mode={davinciWorkspace}
        left={
          isColor ? (
            <ColorDockLeft
              deckSnapshot={playback.deckSnapshot}
              deckARef={playback.deckARef}
              deckBRef={playback.deckBRef}
              mediaPanelProps={mediaPanelProps}
            />
          ) : (
            <EffectDockLeft mediaPanelProps={mediaPanelProps} />
          )
        }
        right={
          isColor ? (
            <ColorDockRight
              selectedClip={timelineOps.selectedClip}
              colorDraft={inspectorOps.colorDraft}
              setColorDraft={inspectorOps.setColorDraft}
              colorWheelsDraft={inspectorOps.colorWheelsDraft}
              setColorWheelsDraft={inspectorOps.setColorWheelsDraft}
              rgbCurvesDraft={{ master: [], red: [], green: [], blue: [] }}
              hslQualifierDraft={{
                enabled: true,
                hue_center: 0,
                hue_width: 30,
                hue_softness: 5,
                sat_center: 0.5,
                sat_width: 0.5,
                sat_softness: 0.1,
                luma_center: 0.5,
                luma_width: 0.5,
                luma_softness: 0.1,
                invert: false,
                blur_radius: 0,
                hue_shift: 0,
                sat_boost: 0,
                exposure_shift: 0,
              }}
              showMatteDraft={false}
              setShowMatteDraft={() => {}}
              updateColorWheels={inspectorOps.setColorWheelsDraft}
              updateRgbCurves={() => {}}
              updateHslQualifier={() => {}}
              applyColorPreset={() => {}}
              applyLutPreset={() => {}}
              loadLutCubeDialog={() => {}}
              saveColor={inspectorOps.saveColor}
              runCommand={async () => {}}
            />
          ) : (
            <EffectDockRight
              selectedClip={timelineOps.selectedClip}
              selectedTimelineClip={selectedTimelineClip}
              addEffect={inspectorOps.addEffect}
              removeEffect={inspectorOps.removeEffect}
              toggleEffect={inspectorOps.toggleEffect}
              reorderEffect={() => {}}
              updateEffectPayload={() => {}}
              loadLutCubeDialog={() => {}}
            />
          )
        }
        center={shellCenter}
        toolbox={shellToolbox}
        timeline={shellTimeline}
      />
    );
  }

  return (
    <WorkspaceShell
      mode="edit"
      left={
        <EditDockLeft mediaPanelProps={mediaPanelProps} trimMonitorProps={trimMonitorProps} />
      }
      right={
        <EditDockRight
          inspectorPanelProps={inspectorPanelProps}
          markersPanelProps={markersPanelProps}
          captionsPanelProps={captionsPanelProps}
          graphicsPanelProps={graphicsPanelProps}
        />
      }
      center={shellCenter}
      toolbox={shellToolbox}
      timeline={shellTimeline}
    />
  );
}
