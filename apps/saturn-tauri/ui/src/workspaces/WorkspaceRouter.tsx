import type { HardwareEncoderInfo } from "../types";
import type { MediaPanelProps } from "../features/media-pool";
import type { MediaHygieneOps } from "../app/useMediaHygiene";
import type { InspectorPanelProps } from "../features/inspector";
import type { MarkersPanelProps } from "../features/markers";
import type { CaptionsPanelProps } from "../features/captions";
import type { NodeGraphPanelProps } from "../features/nodegraph";
import { WorkspaceShell } from "./WorkspaceShell";
import { specialModeDocks } from "./SpecialModeShells";
import { ExportWorkspace } from "./ExportWorkspace";
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
import { invoke } from "@tauri-apps/api/core";
import { TICKS_PER_SECOND, type Clip, type MediaAsset, type ProjectDocument } from "../app/projectTypes";
import type { CommandStatus } from "../app/ProjectProvider";
import type { AudioClockMaster } from "../engine/playbackEngine";
import type { SourcePreviewFocus } from "../features/preview";
import type { CommandSpec } from "../features/menubar/AppHeader";
import type { MarkerOperations } from "../app/useMarkers";
import type { CaptionOperations } from "../app/useCaptions";
import type { NodeGraphOperations } from "../app/useNodeGraph";
import type { GraphicOperations } from "../features/graphics";

export interface WorkspaceRouterProps {
  commands: CommandSpec[];
  encoderInfo: HardwareEncoderInfo | null;
  editorWorkspace: string;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  getMediaUrl: (path: string) => string;
  selectedTimelineClip: Clip | null;
  selectedMedia: MediaAsset | null;
  missingMediaIds: Set<number>;
  busy: boolean;
  timelineWidth: number;
  timelineEndSeconds: number;
  selectEditorWorkspace: (ws: any) => void;
  playback: any;
  mediaOps: any;
  mediaHygiene: MediaHygieneOps;
  timelineOps: any;
  inspectorOps: any;
  projectOps: any;
  invokeMenuCommand: (id: string) => Promise<void>;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  programPreviewError: string | null;
  mediaServerPort: number;
  relinkMedia: (mediaId: number) => Promise<void> | void;
  audioClockMaster: any;
  audioOps: any;
  markerOps: MarkerOperations;
  captionsOps: CaptionOperations;
  nodeGraphOps: NodeGraphOperations;
  graphicsOps: GraphicOperations;
}

export function WorkspaceRouter({
  commands,
  editorWorkspace,
  project,
  media,
  getMediaUrl,
  selectedTimelineClip,
  selectedMedia,
  missingMediaIds,
  busy,
  timelineWidth,
  timelineEndSeconds,
  selectEditorWorkspace,
  playback,
  mediaOps,
  mediaHygiene,
  timelineOps,
  inspectorOps,
  projectOps,
  invokeMenuCommand,
  perform,
  undoStatus,
  redoStatus,
  programPreviewError,
  mediaServerPort,
  relinkMedia,
  audioClockMaster,
  audioOps,
  markerOps,
  captionsOps,
  nodeGraphOps,
  graphicsOps,
  encoderInfo,
}: WorkspaceRouterProps) {
  // Unified monitor Source-tab focus: a click (pointer down→up without drag)
  // on a media-pool item or a timeline clip loads it in the Source tab.
  // Automated selections — media drops, add-to-timeline — must not steal
  // the monitor, so the switch is driven by gestures, not selection state.
  const [sourcePreview, setSourcePreview] = useState<SourcePreviewFocus | null>(null);

  // Deliver workspace live queue state: the embedded batch card renders the
  // real JobManager queue; polling runs only while Deliver is on screen.
  const [deliverPreset, setDeliverPreset] = useState("source");
  const [queuePaused, setQueuePaused] = useState(false);
  const [batchQueue, setBatchQueue] = useState<
    { id: number; name: string; format: string; resolution: string; status: "queued" | "running" | "paused" | "completed" | "failed" | "cancelled"; progress: number; outputPath: string }[]
  >([]);
  useEffect(() => {
    if (editorWorkspace !== "deliver") return;
    let active = true;
    const tick = async () => {
      try {
        const live = await invoke<
          { id: number; label: string; kind: { preset?: string }; state: Record<string, unknown> }[]
        >("get_all_jobs");
        const paused = await invoke<boolean>("is_job_queue_paused");
        if (!active) return;
        setQueuePaused(paused);
        setBatchQueue(
          live.map((job) => {
            const status = String(job.state.status ?? "queued") as
              | "queued"
              | "running"
              | "paused"
              | "completed"
              | "failed"
              | "cancelled";
            const progress = typeof job.state.progress === "number" ? job.state.progress : 0;
            return {
              id: job.id,
              name: job.label,
              format: job.kind?.preset ?? "export",
              resolution: "",
              status,
              progress,
              outputPath: String(job.state.output ?? ""),
            };
          }),
        );
      } catch {
        // Queue probes are best-effort; the card shows the last good list.
      }
    };
    void tick();
    const timer = window.setInterval(() => void tick(), 1000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [editorWorkspace]);

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

  // Deliver is a full-page workspace by user decision (2026-10-23): route it
  // to the previous ExportWorkspace BEFORE the persistent shell. Do not dock
  // it, do not change its layout — the shell modes above/below are unchanged.
  if (editorWorkspace === "deliver") {
    return (
      <ExportWorkspace
        project={project}
        batchQueue={batchQueue}
        setBatchQueue={() => {
          void invoke("clear_job_history")
            .then(() => invoke<never[]>("get_all_jobs"))
            .then((live) => setBatchQueue(live))
            .catch(() => {});
        }}
        queuePaused={queuePaused}
        toggleQueuePause={() =>
          invoke<boolean>(queuePaused ? "resume_job_queue" : "pause_job_queue").then(setQueuePaused)
        }
        deliverDestination="render"
        setDeliverDestination={() => {}}
        handleExportTimelineInterop={projectOps.handleExportTimelineInterop}
        handleImportTimelineInterop={projectOps.handleImportTimelineInterop}
        isInteropBusy={projectOps.isInteropBusy}
        interopContent={projectOps.interopContent}
        setNotice={projectOps.setNotice}
        encoderInfo={encoderInfo}
        selectedExportPreset={deliverPreset}
        setSelectedExportPreset={setDeliverPreset}
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
        addExportToQueue={() => projectOps.enqueueExport(deliverPreset)}
        quickExport={() => projectOps.enqueueExport("source")}
        exportProject={projectOps.exportProject}
      />
    );
  }

  const mediaPanelProps: MediaPanelProps = {
        hygiene: mediaHygiene,
        useProxies: project?.settings.use_proxies ?? false,
        assetTab: mediaOps.assetTab,
        setAssetTab: mediaOps.setAssetTab,
        search: mediaOps.search,
        setSearch: mediaOps.setSearch,
        project,
        media,
        visibleMedia: mediaOps.visibleMedia,
        selectedMediaId: mediaOps.selectedMediaId,
        setSelectedMediaId: (id: number) => mediaOps.setSelectedMediaId(id),
        selectedMediaIds: mediaOps.selectedMediaIds,
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
        onUpdateMetadata: () => mediaOps.updateMediaMetadata(),
        onReplaceSelectedClip: () => mediaOps.replaceSelectedClipMedia(),
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
        mediaServerPort,
        missingMediaIds,
        relinkMedia,
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
        switchMulticamAngle: async (angle: number) => {
          await projectOps.switchMulticamAngle(angle);
        },
        syncMulticamWaveforms: async () => {
          await projectOps.syncMulticamWaveforms();
        },
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
        setSelectedClip: timelineOps.setSelectedClip,
        handleToggleGroupCollapse: timelineOps.handleToggleGroupCollapse,
        handleEditTrackGroup: timelineOps.handleEditTrackGroup,
        handleDeleteTrackGroup: timelineOps.handleDeleteTrackGroup,
        handleAssignTrackToGroup: timelineOps.handleAssignTrackToGroup,
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
        colorDraft: inspectorOps.colorDraft,
        setColorDraft: inspectorOps.setColorDraft,
        saveColor: inspectorOps.saveColor,
        addEffect: inspectorOps.addEffect,
        toggleEffectEnabled: (id) => inspectorOps.toggleEffect(id),
        updateEffectParam: inspectorOps.updateEffectParam,
        saveEffectParam: inspectorOps.saveEffectParam,
        removeEffect: (id) => inspectorOps.removeEffect(id),
        reorderEffect: inspectorOps.reorderEffects,
        updateEffectPayload: inspectorOps.updateEffectPayload,
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
        selectEditorWorkspace,
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
        saveClipVolumeDb: inspectorOps.saveClipVolumeDb,
        frameFit: inspectorOps.frameFit,
        makeSubclip: inspectorOps.makeSubclip,
        flattenMulticam: inspectorOps.flattenMulticam,
        revealClipMedia: () => void invokeMenuCommand("media.reveal"),
        releaseNestedSequence: () => inspectorOps.releaseNestedSequence(),
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
        onRipple: (deltaTicks: number) => void markerOps.rippleMarkers(deltaTicks),
        onAdd: markerOps.addMarker,
        onAddRange: markerOps.addRangeMarker,
        onImport: markerOps.importMarkers,
        onExport: markerOps.exportMarkers,
  };  const captionsPanelProps: CaptionsPanelProps = {
        track: captionsOps.track,
        captionTracks: captionsOps.captionTracks,
        activeTrackId: captionsOps.activeTrackId,
        onTrackChange: (id: number) => captionsOps.setActiveTrackId(id),
        onTrackAdd: () => void captionsOps.addCaptionTrack(),
        onTrackRename: (id: number, name: string) => void captionsOps.renameCaptionTrack(id, name),
        onTrackRemove: (id: number) => void captionsOps.removeCaptionTrack(id),
        onToggleTrackVisible: (id: number, visible: boolean) =>
          void captionsOps.toggleTrackVisible(id, visible),
        onShowAllTracks: () => void captionsOps.showAllTracks(),
        onHideAllTracks: () => void captionsOps.hideAllTracks(),
        style: captionsOps.style,
        onSaveStyle: (patch) => void captionsOps.saveCaptionStyle(patch),
        cues: captionsOps.cues,
        playhead: playback.playhead,
        frameRate: project?.settings.frame_rate ?? { numerator: 30, denominator: 1 },
        selectedCueIds: captionsOps.selectedCueIds,
        revealTick: captionsOps.revealTick,
        onSelect: captionsOps.selectCue,
        onSelectAll: captionsOps.selectAllCues,
        onAdd: captionsOps.addCue,
        onEditText: captionsOps.updateCueText,
        onEditSpeaker: captionsOps.updateCueSpeaker,
        onNudge: captionsOps.nudgeCue,
        onDelete: captionsOps.removeCue,
        onDeleteSelected: () => void captionsOps.deleteSelectedCues(),
        onSplit: () => void captionsOps.splitCueAtPlayhead(),
        onMerge: () => void captionsOps.mergeSelectedCues(),
        onNext: () => void captionsOps.nextCue(),
        onPrevious: () => void captionsOps.previousCue(),
        onImport: captionsOps.importCues,
        onExport: captionsOps.exportCues,
  };  const graphicsPanelProps: { ops: GraphicOperations; playhead: number; frame: { width: number; height: number } } = {
        ops: graphicsOps,
        playhead: playback.playhead,
        frame: project?.settings ?? { width: 1920, height: 1080 },
  };
  const nodeGraphPanelProps: NodeGraphPanelProps = {
        graph: nodeGraphOps.graph,
        onAddNode: (kind) => void nodeGraphOps.addNode(kind),
        onRemoveNode: (id: number) => void nodeGraphOps.removeNode(id),
        onConnect: (from: number, fromSocket: string, to: number, toSocket: string) =>
          void nodeGraphOps.connectNodes(from, fromSocket, to, toSocket),
        onDisconnect: (conn) => void nodeGraphOps.disconnectNodes(conn.from_node, conn.from_socket, conn.to_node, conn.to_socket),
        onUpdateNode: (id: number, patch) => void nodeGraphOps.updateNode(id, patch),
        onAddMask: () => void nodeGraphOps.addMask(),
  };
  const shellToolbox = (
    <TimelineToolRail
      timelineTool={timelineSectionProps.timelineTool}
      setTimelineTool={timelineSectionProps.setTimelineTool}
    />
  );
  const shellTimeline = <TimelineSection {...timelineSectionProps} />;
  const shellCenter = <PreviewSection {...previewSectionProps} />;
  const isShellWorkspace =
    editorWorkspace === "media" ||
    editorWorkspace === "fusion" ||
    editorWorkspace === "audio" ||
    editorWorkspace === "keyframes";
  if (isShellWorkspace) {
    const docks = specialModeDocks({
      editorWorkspace,
      project,
      media,
      selectedMedia,
      selectedTimelineClip,
      busy,
      missingMediaIds,
      timelineWidth,
      selectEditorWorkspace,
      playback,
      mediaOps,
      timelineOps,
      inspectorOps,
      projectOps,
      audioOps,
      audioClockMaster,
      invokeMenuCommand,
      perform,
      panelProps: {
        media: mediaPanelProps,
        inspector: inspectorPanelProps,
        markers: markersPanelProps,
        captions: captionsPanelProps,
        graphics: graphicsPanelProps,
        nodeGraph: nodeGraphPanelProps,
      },
    });
    return (
      <WorkspaceShell
        mode={editorWorkspace}
        left={docks.left}
        right={docks.right}
        center={shellCenter}
        toolbox={shellToolbox}
        timeline={shellTimeline}
      />
    );
  }

  if (editorWorkspace === "color" || editorWorkspace === "effects") {
    const isColor = editorWorkspace === "color";
    return (
      <WorkspaceShell
        mode={editorWorkspace}
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
              colorDraft={inspectorOps.colorDraft}
              setColorDraft={inspectorOps.setColorDraft}
              colorWheelsDraft={inspectorOps.colorWheelsDraft}
              setColorWheelsDraft={inspectorOps.setColorWheelsDraft}
              rgbCurvesDraft={inspectorOps.rgbCurvesDraft}
              hslQualifierDraft={inspectorOps.hslQualifierDraft}
              showMatteDraft={inspectorOps.showMatteDraft}
              setShowMatteDraft={inspectorOps.setShowMatteDraft}
              updateColorWheels={inspectorOps.setColorWheelsDraft}
              updateRgbCurves={inspectorOps.saveRgbCurves}
              updateHslQualifier={inspectorOps.saveHslQualifier}
              applyColorPreset={() => {}}
              applyLutPreset={() => {}}
              loadLutCubeDialog={() => {}}
              saveColor={inspectorOps.saveColor}
              runCommand={inspectorOps.runCommand}
            />
          ) : (
            <EffectDockRight
              selectedTimelineClip={selectedTimelineClip}
              addEffect={inspectorOps.addEffect}
              removeEffect={inspectorOps.removeEffect}
              toggleEffect={inspectorOps.toggleEffect}
              reorderEffect={inspectorOps.reorderEffects}
              updateEffectPayload={inspectorOps.updateEffectPayload}
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
          nodeGraphPanelProps={nodeGraphPanelProps}
        />
      }
      center={shellCenter}
      toolbox={shellToolbox}
      timeline={shellTimeline}
    />
  );
}
