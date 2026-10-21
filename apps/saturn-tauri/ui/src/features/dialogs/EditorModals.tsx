import { NewProjectModal } from "./NewProjectModal";
import { TranscriptionModal } from "./TranscriptionModal";
import { SilenceRemovalModal } from "./SilenceRemovalModal";
import { InteropModal } from "./InteropModal";
import { CompoundClipModal } from "./CompoundClipModal";
import { SpeedDurationModal } from "./SpeedDurationModal";
import { GpuTelemetryModal } from "./GpuTelemetryModal";
import { MediaBinModal } from "./MediaBinModal";
import { MediaPropsModal } from "./MediaPropsModal";
import { NetworkImportModal } from "../../components/NetworkImportModal";
import { BatchQueueDrawer } from "../../components/BatchQueueDrawer";
import { OllamaChatDrawer } from "../../components/OllamaChatDrawer";
import type { MediaAsset, ProjectDocument } from "../../app/projectTypes";

export interface EditorModalsProps {
  projectOps: any;
  mediaOps: any;
  timelineOps: any;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  busy: boolean;
  refreshProject: () => Promise<unknown>;
  setNotice: (msg: string) => void;
}

export function EditorModals({
  projectOps,
  mediaOps,
  timelineOps,
  project,
  media,
  busy,
  refreshProject,
  setNotice,
}: EditorModalsProps) {
  const selected = timelineOps.selectedClip;
  const selectedClip = selected
    ? (project?.sequence.tracks
        .find((track) => track.id === selected.track_id)
        ?.clips[selected.clip_index] ?? null)
    : null;
  const selectedClipMediaName = selectedClip
    ? (media.find((asset) => asset.id === selectedClip.media_id)?.name ?? "")
    : "";

  return (
    <>
      <NewProjectModal
        isOpen={projectOps.newProjectOpen}
        onClose={() => projectOps.setNewProjectOpen(false)}
        name={projectOps.newProjectName}
        setName={projectOps.setNewProjectName}
        onCreate={projectOps.createProject}
      />

      <TranscriptionModal
        isOpen={projectOps.transcribeModalOpen}
        onClose={() => projectOps.setTranscribeModalOpen(false)}
        transcribeModel={projectOps.transcribeModel}
        setTranscribeModel={projectOps.setTranscribeModel}
        transcribeLanguage={projectOps.transcribeLanguage}
        setTranscribeLanguage={projectOps.setTranscribeLanguage}
        transcribeTextOverride={projectOps.transcribeTextOverride}
        setTranscribeTextOverride={projectOps.setTranscribeTextOverride}
        transcribeReplaceExisting={projectOps.transcribeReplaceExisting}
        setTranscribeReplaceExisting={projectOps.setTranscribeReplaceExisting}
        isTranscribing={projectOps.isTranscribing}
        handleAutoTranscribe={projectOps.handleAutoTranscribe}
      />

      <SilenceRemovalModal
        isOpen={projectOps.silenceModalOpen}
        onClose={() => projectOps.setSilenceModalOpen(false)}
        silenceThresholdDb={projectOps.silenceThresholdDb}
        setSilenceThresholdDb={projectOps.setSilenceThresholdDb}
        silenceMinDurationMs={projectOps.silenceMinDurationMs}
        setSilenceMinDurationMs={projectOps.setSilenceMinDurationMs}
        silencePaddingMs={projectOps.silencePaddingMs}
        setSilencePaddingMs={projectOps.setSilencePaddingMs}
        silenceIntervalsLength={projectOps.silenceIntervals.length}
        isDetectingSilence={projectOps.isDetectingSilence}
        handleDetectSilence={projectOps.handleDetectSilence}
        handleRippleDeleteFillers={projectOps.handleRippleDeleteFillers}
        handleRippleDeleteSilence={projectOps.handleRippleDeleteSilence}
        busy={busy}
      />

      <InteropModal
        isOpen={projectOps.interopModalOpen}
        onClose={() => projectOps.setInteropModalOpen(false)}
        interopMode={projectOps.interopMode}
        interopFormat={projectOps.interopFormat}
        setInteropFormat={projectOps.setInteropFormat}
        interopContent={projectOps.interopContent}
        setInteropContent={projectOps.setInteropContent}
        interopProjectName={projectOps.interopProjectName}
        setInteropProjectName={projectOps.setInteropProjectName}
        isInteropBusy={projectOps.isInteropBusy}
        setIsInteropBusy={projectOps.setIsInteropBusy}
        handleExportTimelineInterop={projectOps.handleExportTimelineInterop}
        handleImportTimelineInterop={projectOps.handleImportTimelineInterop}
        refreshProject={async () => {
          await refreshProject();
        }}
        setNotice={setNotice}
      />

      <CompoundClipModal
        isOpen={projectOps.compoundClipModalOpen}
        onClose={() => projectOps.setCompoundClipModalOpen(false)}
        compoundClipNameDraft={projectOps.compoundClipNameDraft}
        setCompoundClipNameDraft={projectOps.setCompoundClipNameDraft}
        createCompoundClip={timelineOps.createCompoundClip}
      />

      <SpeedDurationModal
        isOpen={projectOps.speedDurationOpen}
        onClose={() => projectOps.setSpeedDurationOpen(false)}
        clip={selectedClip}
        mediaName={selectedClipMediaName}
        onApply={timelineOps.applySpeedDuration}
      />

      <GpuTelemetryModal
        isOpen={projectOps.gpuTelemetryOpen}
        onClose={() => projectOps.setGpuTelemetryOpen(false)}
        gpuMetrics={projectOps.gpuMetrics}
      />

      <MediaBinModal
        isOpen={mediaOps.createBinModalOpen}
        onClose={() => mediaOps.setCreateBinModalOpen(false)}
        newBinName={mediaOps.newBinName}
        setNewBinName={mediaOps.setNewBinName}
        newBinColor={mediaOps.newBinColor}
        setNewBinColor={mediaOps.setNewBinColor}
        createMediaBin={mediaOps.createMediaBin}
      />

      <MediaPropsModal
        mediaEditPropsId={mediaOps.mediaEditPropsId}
        onClose={() => mediaOps.setMediaEditPropsId(null)}
        mediaEditPropsName={mediaOps.mediaEditPropsName}
        setMediaEditPropsName={mediaOps.setMediaEditPropsName}
        mediaEditPropsColor={mediaOps.mediaEditPropsColor}
        setMediaEditPropsColor={mediaOps.setMediaEditPropsColor}
        mediaEditPropsTags={mediaOps.mediaEditPropsTags}
        setMediaEditPropsTags={mediaOps.setMediaEditPropsTags}
        media={media}
        project={project}
        moveMediaToBin={mediaOps.moveMediaToBin}
        updateMediaProperties={mediaOps.updateMediaProperties}
      />

      <NetworkImportModal
        isOpen={mediaOps.networkImportModalOpen}
        onClose={() => mediaOps.setNetworkImportModalOpen(false)}
        onSuccess={() => void refreshProject()}
      />

      <BatchQueueDrawer
        isOpen={mediaOps.batchQueueDrawerOpen}
        onClose={() => mediaOps.setBatchQueueDrawerOpen(false)}
        onEnqueueCurrentTimeline={(preset) => {
          setNotice(`Enqueued timeline with preset "${preset}" to batch render daemon`);
        }}
      />

      <OllamaChatDrawer
        isOpen={mediaOps.aiChatDrawerOpen}
        onClose={() => mediaOps.setAiChatDrawerOpen(false)}
        onExecuteAction={(key) => setNotice(`Executed AI Action: ${key}`)}
      />
    </>
  );
}
