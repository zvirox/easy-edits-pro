import { MediaWorkspace } from "./MediaWorkspace";
import { ExportWorkspace } from "./ExportWorkspace";
import { FairlightWorkspace } from "./FairlightWorkspace";
import { FusionWorkspace } from "./FusionWorkspace";
import { ColorWorkspace } from "./ColorWorkspace";
import { KeyframesWorkspace } from "./KeyframesWorkspace";
import { EffectsWorkspace } from "./EffectsWorkspace";
import { EditWorkspace } from "./EditWorkspace";
import type { Clip, MediaAsset, ProjectDocument } from "../app/projectTypes";
import type { CommandStatus } from "../app/ProjectProvider";
import type { AudioClockMaster } from "../engine/playbackEngine";

export interface WorkspaceRouterProps {
  davinciWorkspace: string;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
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
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  programPreviewError: string | null;
  audioClockMaster: any;
}

export function WorkspaceRouter({
  davinciWorkspace,
  project,
  media,
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
  perform,
  undoStatus,
  redoStatus,
  programPreviewError,
  audioClockMaster,
}: WorkspaceRouterProps) {
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
        setNotice={() => {}}
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
        chooseExportDestination={async () => {}}
        useHardwareEncoding={projectOps.useHardwareEncoding}
        setUseHardwareEncoding={projectOps.setUseHardwareEncoding}
        exportMarkedRange={projectOps.exportMarkedRange}
        setExportMarkedRange={projectOps.setExportMarkedRange}
        invokeMenuCommand={projectOps.invokeMenuCommand}
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
        selectedFairlightTrackId={null}
        setSelectedFairlightTrackId={() => {}}
        audioProcessingDrafts={{}}
        setAudioProcessingDrafts={() => {}}
        saveTrackAudioProcessing={async () => {}}
        trackGr={{}}
        masterGr={0}
        vuLevels={{}}
        masterVu={{ left: -60, right: -60, peakDb: -60 }}
        gainDrafts={timelineOps.gainDrafts}
        setGainDrafts={timelineOps.setGainDrafts}
        saveTrackGain={timelineOps.saveTrackGain}
        panDrafts={timelineOps.panDrafts}
        setPanDrafts={timelineOps.setPanDrafts}
        saveTrackPan={timelineOps.saveTrackPan}
        masterGainDraft={0}
        setMasterGainDraft={() => {}}
        busGainDrafts={{}}
        setBusGainDrafts={() => {}}
        loudnessStandard="ebu_r128"
        setLoudnessStandard={() => {}}
        loudnessReport={{
          integratedLufs: -70,
          shortTermLufs: -70,
          momentaryLufs: -70,
          loudnessRangeLu: 0,
          truePeakDbtp: -60,
          targetLufs: -23,
          isCompliant: true,
          gainRecommendationDb: 0,
        }}
        autoTrimLoudness={() => {}}
        resetLoudness={() => {}}
        programPlaying={playback.programPlaying}
        toggleProgramPlayback={playback.toggleProgramPlayback}
        stepProgramFrame={playback.stepProgramFrame}
        handleShuttleKey={playback.handleShuttleKey}
        clearAllSolos={() => {}}
        muteAllTracks={() => {}}
        toggleTrackMute={timelineOps.toggleTrackMute}
        toggleTrackSolo={timelineOps.toggleTrackSolo}
        toggleTrackLock={timelineOps.toggleTrackLock}
        setTrackBusRoute={() => {}}
        setTrackAutomationMode={() => {}}
        addTrackPlugin={() => {}}
        toggleTrackPlugin={() => {}}
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

  if (davinciWorkspace === "color") {
    return (
      <ColorWorkspace
        project={project}
        media={media}
        selectedClip={timelineOps.selectedClip}
        selectedTimelineClip={selectedTimelineClip}
        setSelectedClip={timelineOps.setSelectedClip}
        setSelectedClips={timelineOps.setSelectedClips}
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
        colorStudioTab="wheels"
        setColorStudioTab={() => {}}
        updateColorWheels={inspectorOps.setColorWheelsDraft}
        updateRgbCurves={() => {}}
        updateHslQualifier={() => {}}
        applyColorPreset={() => {}}
        applyLutPreset={() => {}}
        loadLutCubeDialog={() => {}}
        saveColor={inspectorOps.saveColor}
        runCommand={async () => {}}
        selectDavinciWorkspace={selectDavinciWorkspace}
        playhead={playback.playhead}
        renderProgramMonitor={() => null}
        programPlaying={playback.programPlaying}
        toggleProgramPlayback={playback.toggleProgramPlayback}
        stepProgramFrame={playback.stepProgramFrame}
        seekProgramTo={playback.seekProgramTo}
        deckSnapshot={playback.deckSnapshot}
        deckARef={playback.deckARef}
        deckBRef={playback.deckBRef}
      />
    );
  }

  if (davinciWorkspace === "effects") {
    return (
      <EffectsWorkspace
        project={project}
        media={media}
        selectedClip={timelineOps.selectedClip}
        selectedTimelineClip={selectedTimelineClip}
        setSelectedClip={timelineOps.setSelectedClip}
        setSelectedClips={timelineOps.setSelectedClips}
        addEffect={inspectorOps.addEffect}
        removeEffect={inspectorOps.removeEffect}
        toggleEffect={inspectorOps.toggleEffect}
        reorderEffect={() => {}}
        updateEffectPayload={() => {}}
        loadLutCubeDialog={() => {}}
        selectDavinciWorkspace={selectDavinciWorkspace}
        playhead={playback.playhead}
        renderProgramMonitor={() => null}
        programPlaying={playback.programPlaying}
        toggleProgramPlayback={playback.toggleProgramPlayback}
        stepProgramFrame={playback.stepProgramFrame}
        seekProgramTo={playback.seekProgramTo}
      />
    );
  }

  if (davinciWorkspace === "keyframes") {
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
        selectDavinciWorkspace={selectDavinciWorkspace}
      />
    );
  }

  return (
    <EditWorkspace
      mediaPanelProps={{
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
        deleteBin: mediaOps.deleteMediaBin,
        beginMediaPointer: mediaOps.beginMediaPointer,
        moveMediaPointer: mediaOps.moveMediaPointer,
        cancelMediaPointer: mediaOps.cancelMediaPointer,
        playhead: playback.playhead,
      }}
      previewSectionProps={{
        selectedMedia,
        getMediaUrl: playback.getMediaUrl,
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
      }}
      timelineSectionProps={{
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
        selectedClip: timelineOps.selectedClip,
        splitSelectedClip: timelineOps.splitSelectedClip,
        rippleDeleteSelectedClip: timelineOps.rippleDeleteSelectedClip,
        removeSelectedClip: timelineOps.removeSelectedClip,
        selectedClips: timelineOps.selectedClips,
        linkSelectedClips: timelineOps.linkSelectedClips,
        unlinkSelectedClips: timelineOps.unlinkSelectedClips,
        setCompoundClipNameDraft: projectOps.setCompoundClipNameDraft,
        setCompoundClipModalOpen: projectOps.setCompoundClipModalOpen,
        decomposeSelectedCompoundClip: timelineOps.decomposeSelectedCompoundClip,
        invokeMenuCommand: projectOps.invokeMenuCommand,
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
        setSelectedClipId: timelineOps.setSelectedClipId,
        setSelectedClip: timelineOps.setSelectedClip,
        handleToggleGroupCollapse: timelineOps.handleToggleGroupCollapse,
        handleEditTrackGroup: timelineOps.handleEditTrackGroup,
        toggleTrackLock: timelineOps.toggleTrackLock,
        toggleTrackMute: timelineOps.toggleTrackMute,
        toggleTrackSolo: timelineOps.toggleTrackSolo,
        gainDrafts: timelineOps.gainDrafts,
        setGainDrafts: timelineOps.setGainDrafts,
        saveTrackGain: timelineOps.saveTrackGain,
        audioClockMaster,
        panDrafts: timelineOps.panDrafts,
        setPanDrafts: timelineOps.setPanDrafts,
        saveTrackPan: timelineOps.saveTrackPan,
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
      }}
      inspectorPanelProps={{
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
        invokeMenuCommand: projectOps.invokeMenuCommand,
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
      }}
    />
  );
}
