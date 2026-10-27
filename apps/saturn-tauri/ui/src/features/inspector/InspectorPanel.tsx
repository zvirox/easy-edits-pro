import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction } from "react";
import { StudioPanel } from "../../components/StudioPanel";
import { AudioMonitorMixer } from "../../components/AudioMonitorMixer";
import { SubtitleInspector } from "./SubtitleInspector";
import { EffectsInspector } from "./EffectsInspector";
import { KeyframesInspector } from "./KeyframesInspector";
import { TransitionInspector } from "./TransitionInspector";
import { TitleInspector } from "./TitleInspector";
import { ClipInspector } from "./ClipInspector";
import { useClipSelection } from "../../app/selectionStore";
import { LoadingButton } from "../../components/interior/loading-button";
import { formatTime } from "../../utils/formatters";
import type {
  Clip,
  ColorAdjustments,
  MediaAsset,
  ProjectDocument,
  SubtitleItem,
  TitleProperties,
  Transition,
  TransitionKind,
  Track,
} from "../../app/projectTypes";
import type {
  AudioClockMaster,
  EffectPayload,
  EffectKind,
  InterpolationKind,
  KeyframeProperty,
  MotionBlurParams,
  SpeedRamp,
  TrackingMode,
} from "../../engine/playbackEngine";

export interface InspectorPanelProps {
  workspace: string;
  selectedSubtitle: SubtitleItem | null;
  setSelectedSubtitle: (sub: SubtitleItem | null) => void;
  subtitleTextDraft: string | null;
  setSubtitleTextDraft: (text: string | null) => void;
  updateSubtitle: (id: number, patch: Partial<SubtitleItem>) => Promise<void>;
  addSubtitleAtPlayhead: (startTicks?: number) => Promise<void>;
  removeSubtitle: (id: number) => Promise<void>;
  clearSubtitles: () => Promise<void>;
  importSubtitlesDialog: () => Promise<void>;
  exportSubtitlesDialog: () => Promise<void>;
  setTranscribeModalOpen: (open: boolean) => void;
  setSilenceModalOpen: (open: boolean) => void;
  handleDetectSilence: () => Promise<void>;

  selectedTimelineClip: Clip | null;
  colorDraft: ColorAdjustments;
  setColorDraft: Dispatch<SetStateAction<ColorAdjustments>>;
  saveColor: (adjustments: ColorAdjustments) => Promise<void>;

  addEffect: (kind: EffectKind) => Promise<void>;
  toggleEffectEnabled: (id: string) => Promise<void>;
  updateEffectParam: (id: string, paramKey: string, value: number) => void;
  saveEffectParam: (id: string, paramKey: string, value: number) => Promise<void>;
  removeEffect: (id: string) => Promise<void>;

  playhead: number;
  keyframeProperty: KeyframeProperty;
  setKeyframeProperty: (prop: KeyframeProperty) => void;
  keyframeValue: string;
  setKeyframeValue: (val: string) => void;
  keyframeInterpDraft: InterpolationKind;
  setKeyframeInterpDraft: (kind: InterpolationKind) => void;
  keyframeHandleInX: number;
  keyframeHandleInY: number;
  keyframeHandleOutX: number;
  keyframeHandleOutY: number;
  selectedKeyframeTime: number | null;
  setSelectedKeyframeTime: (time: number | null) => void;
  selectEditorWorkspace: (ws: any) => void;
  saveKeyframe: (
    property: KeyframeProperty,
    value: number,
    time?: number,
    interpolation?: InterpolationKind,
    hInX?: number,
    hInY?: number,
    hOutX?: number,
    hOutY?: number
  ) => Promise<void>;
  deleteKeyframe: (property: KeyframeProperty, time: number) => Promise<void>;

  project: ProjectDocument["project"] | null;
  exportMarkedRange: boolean;
  setExportMarkedRange: (val: boolean) => void;
  invokeMenuCommand: (cmd: string) => Promise<void>;
  exportFormat: "mp4" | "webm";
  setExportFormat: (fmt: "mp4" | "webm") => void;
  rendering: boolean;
  exportProject: () => Promise<void>;
  renderProgress: { fractionPercent: number; phase: string } | null;

  assetQuery: string;
  setAssetQuery: (q: string) => void;
  assetType: "images" | "audio";
  setAssetType: (t: "images" | "audio") => void;
  busy: boolean;
  searchAssets: () => Promise<void>;
  assetResults: any[];
  addLibraryAsset: (asset: any) => Promise<void>;

  aiModel: string;
  setAiModel: (m: string) => void;
  aiPrompt: string;
  setAiPrompt: (p: string) => void;
  askAi: () => Promise<void>;
  aiReply: string;

  selectedTransition: { track_id: number; transition: Transition } | null;
  setSelectedTransition: Dispatch<SetStateAction<{ track_id: number; transition: Transition } | null>>;
  updateTransition: (kind: TransitionKind, duration: number) => Promise<void>;
  removeSelectedTransition: () => Promise<void>;

  media: MediaAsset[];
  titleDraft: TitleProperties | null;
  setTitleDraft: Dispatch<SetStateAction<TitleProperties | null>>;
  saveTitleProperty: (patch: Partial<TitleProperties>) => Promise<void>;
  removeSelectedClip: () => void;

  selectedMedia: MediaAsset | null;
  unlinkSelectedClips: () => Promise<void>;
  trackingMode: TrackingMode;
  setTrackingMode: (mode: TrackingMode) => void;
  showTrackingBox: boolean;
  setShowTrackingBox: Dispatch<SetStateAction<boolean>>;
  trackingSmoothness: number;
  updateStabilizationSmoothness: (val: number) => Promise<void>;
  trackingTargetClipId: number | null;
  pinClipToMotionTrack: (targetId: number | null) => Promise<void>;
  isAnalyzingTracking: boolean;
  runMotionTracker: (forward: boolean) => Promise<void>;
  saveSpeedRamp: (speedRamp: SpeedRamp) => Promise<void>;
  saveMotionBlur: (motionBlur: MotionBlurParams) => Promise<void>;
  saveClipVolumeDb: (volumeDb: number) => Promise<void>;
  frameFit: (mode: "fill" | "fit" | "scale100") => Promise<void>;
  makeSubclip: () => Promise<void>;
  flattenMulticam: () => Promise<void>;
  revealClipMedia: () => void;
  releaseNestedSequence: () => Promise<void>;

  perform?: (cmd: string, params?: unknown) => Promise<unknown>;
  updateEffectPayload?: (id: number, payload: EffectPayload) => Promise<void>;
  reorderEffect?: (fromIndex: number, toIndex: number) => Promise<void>;

  // Audio mixer props
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
  setMasterGainDraft: (gain: number) => void;
  gainDrafts: Record<number, number>;
  setGainDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackGain: (trackId: number, gain: number) => Promise<void>;
  toggleTrackMute: (track: Track) => Promise<void> | void;
  toggleTrackSolo: (track: Track) => Promise<void> | void;
  audioClockMaster: AudioClockMaster;
  activeClip?: { track: Track; clip: Clip; media: MediaAsset } | Clip | null;
}

export function InspectorPanel(props: InspectorPanelProps) {
  // F1: selection comes from the shared store; the prop is gone.
  const { clip: selectedClip } = useClipSelection();
  function renderInspectorContent() {
    if (props.selectedSubtitle) {
      return (
        <SubtitleInspector
          selectedSubtitle={props.selectedSubtitle}
          setSelectedSubtitle={props.setSelectedSubtitle}
          subtitleTextDraft={props.subtitleTextDraft}
          setSubtitleTextDraft={props.setSubtitleTextDraft}
          updateSubtitle={props.updateSubtitle}
          perform={props.perform ?? (async () => {})}
          removeSubtitle={props.removeSubtitle}
          clearSubtitles={props.clearSubtitles}
          importSubtitlesDialog={props.importSubtitlesDialog}
          exportSubtitlesDialog={props.exportSubtitlesDialog}
          setTranscribeModalOpen={props.setTranscribeModalOpen}
          setSilenceModalOpen={props.setSilenceModalOpen}
          handleDetectSilence={props.handleDetectSilence}
        />
      );
    }

    switch (props.workspace) {
      case "color": {
        const updateColorField = (field: keyof ColorAdjustments, value: number) => {
          const next = { ...props.colorDraft, [field]: value };
          props.setColorDraft(next);
          void props.saveColor(next);
        };
        return (
          <>
            <h2>Color grading</h2>
            <p>{props.selectedTimelineClip ? "Adjust the selected timeline clip." : "Select a timeline clip to grade it."}</p>
            <label>
              Exposure
              <input
                type="range"
                min="-5"
                max="5"
                step="0.1"
                disabled={!selectedClip}
                value={props.colorDraft.exposure}
                onChange={(event) => props.setColorDraft((val) => ({ ...val, exposure: Number(event.target.value) }))}
                onPointerUp={(event) => updateColorField("exposure", Number(event.currentTarget.value))}
                onKeyUp={(event) => updateColorField("exposure", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Contrast
              <input
                type="range"
                min="0"
                max="2"
                step="0.01"
                disabled={!selectedClip}
                value={props.colorDraft.contrast}
                onChange={(event) => props.setColorDraft((val) => ({ ...val, contrast: Number(event.target.value) }))}
                onPointerUp={(event) => updateColorField("contrast", Number(event.currentTarget.value))}
                onKeyUp={(event) => updateColorField("contrast", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Saturation
              <input
                type="range"
                min="0"
                max="2"
                step="0.01"
                disabled={!selectedClip}
                value={props.colorDraft.saturation}
                onChange={(event) => props.setColorDraft((val) => ({ ...val, saturation: Number(event.target.value) }))}
                onPointerUp={(event) => updateColorField("saturation", Number(event.currentTarget.value))}
                onKeyUp={(event) => updateColorField("saturation", Number(event.currentTarget.value))}
              />
            </label>
            <small>Preview uses webview color filters. Values are saved with the project.</small>
          </>
        );
      }
      case "effects":
        return (
          <EffectsInspector
            selectedTimelineClip={props.selectedTimelineClip}
            selectedClip={selectedClip}
            addEffect={props.addEffect}
            toggleEffect={async (id) => props.toggleEffectEnabled(String(id))}
            reorderEffect={(fromIndex, toIndex) =>
              props.reorderEffect?.(fromIndex, toIndex) ?? Promise.resolve()
            }
            removeEffect={(id) => props.removeEffect(String(id))}
            updateEffectPayload={props.updateEffectPayload ?? (async () => {})}
            loadLutCubeDialog={async () => {}}
          />
        );
      case "keyframes":
        return (
          <KeyframesInspector
            selectedTimelineClip={props.selectedTimelineClip}
            selectedClip={selectedClip}
            playhead={props.playhead}
            keyframeProperty={props.keyframeProperty}
            setKeyframeProperty={props.setKeyframeProperty}
            keyframeValue={props.keyframeValue}
            setKeyframeValue={props.setKeyframeValue}
            keyframeInterpDraft={props.keyframeInterpDraft}
            setKeyframeInterpDraft={props.setKeyframeInterpDraft}
            keyframeHandleInX={props.keyframeHandleInX}
            keyframeHandleInY={props.keyframeHandleInY}
            keyframeHandleOutX={props.keyframeHandleOutX}
            keyframeHandleOutY={props.keyframeHandleOutY}
            selectedKeyframeTime={props.selectedKeyframeTime}
            setSelectedKeyframeTime={props.setSelectedKeyframeTime}
            selectEditorWorkspace={props.selectEditorWorkspace}
            saveKeyframe={props.saveKeyframe}
            deleteKeyframe={props.deleteKeyframe}
          />
        );
      case "export":
        return (
          <>
            <h2>Export</h2>
            <p>Render the sequence using its project resolution and frame rate.</p>
            <div className="control">
              In point <span>{props.project?.sequence.mark_in == null ? "Not set" : formatTime(props.project.sequence.mark_in)}</span>
            </div>
            <div className="control">
              Out point <span>{props.project?.sequence.mark_out == null ? "Not set" : formatTime(props.project.sequence.mark_out)}</span>
            </div>
            <label className="range-export">
              <input
                type="checkbox"
                checked={props.exportMarkedRange}
                disabled={props.project?.sequence.mark_in == null || props.project?.sequence.mark_out == null}
                onChange={(event) => props.setExportMarkedRange(event.target.checked)}
              />{" "}
              Export marked In/Out range
            </label>
            <InteriorButton
              disabled={props.project?.sequence.mark_in == null && props.project?.sequence.mark_out == null}
              onClick={() => void props.invokeMenuCommand("timeline.clear_marks")}
            >
              Clear marks
            </InteriorButton>
            <label>
              Format
              <select
                value={props.exportFormat}
                disabled={props.rendering}
                onChange={(event) => props.setExportFormat(event.target.value as "mp4" | "webm")}
              >
                <option value="mp4">MP4 · H.264 + AAC</option>
                <option value="webm">WebM · VP8 + Vorbis</option>
              </select>
            </label>
            <div className="control">
              Resolution <span>{props.project ? `${props.project.settings.width} × ${props.project.settings.height}` : "—"}</span>
            </div>
            <div className="control">
              Frame rate <span>{props.project ? `${props.project.settings.frame_rate.numerator}/${props.project.settings.frame_rate.denominator} fps` : "—"}</span>
            </div>
            <InteriorButton
              className="primary"
              disabled={!props.project || props.rendering || !props.project.sequence.tracks.some((track) => track.clips.length > 0)}
              onClick={() => void props.exportProject()}
            >
              {props.rendering ? "Rendering…" : "Render video"}
            </InteriorButton>
            {props.renderProgress && (
              <div className="render-progress">
                <progress max={100} value={props.renderProgress.fractionPercent} />
                <small>
                  {props.renderProgress.phase} · {props.renderProgress.fractionPercent}%
                </small>
              </div>
            )}
            <small>
              MP4 prefers an installed VA-API, Quick Sync, NVENC, or V4L2 H.264 encoder; software encoding is used when hardware is unavailable. Color controls and track gain are included; animated keyframes are not yet rendered.
            </small>
          </>
        );
      case "library":
        return (
          <>
            <h2>FOSS asset library</h2>
            <p>Search openly licensed media indexed by Openverse. Check each source page and license before publishing your work.</p>
            <label>
              Search{" "}
              <input
                value={props.assetQuery}
                maxLength={200}
                placeholder="Try nature, city, music…"
                onChange={(event) => props.setAssetQuery(event.target.value)}
              />
            </label>
            <label>
              Media type{" "}
              <select value={props.assetType} onChange={(event) => props.setAssetType(event.target.value as typeof props.assetType)}>
                <option value="images">Images</option>
                <option value="audio">Audio</option>
              </select>
            </label>
            <LoadingButton
              className="primary"
              disabled={props.busy || !props.assetQuery.trim()}
              onAction={props.searchAssets}
              pendingLabel="Searching…"
              successLabel="Search complete"
              errorLabel="Search failed"
            >
              Search Openverse
            </LoadingButton>
            <div className="asset-results">
              {props.assetResults.map((asset) => (
                <article className="asset-result" key={`${asset.id}-${asset.mediaUrl}`}>
                  {asset.thumbnailUrl && <img src={asset.thumbnailUrl} alt="" loading="lazy" />}
                  <strong>{asset.title}</strong>
                  <small>{asset.creator ? `By ${asset.creator}` : "Creator not listed"} · {asset.license ?? "Check source license"}</small>
                  <div className="asset-actions">
                    <a href={asset.sourceUrl} target="_blank" rel="noreferrer">
                      Source and license
                    </a>
                    <InteriorButton disabled={props.busy} onClick={() => void props.addLibraryAsset(asset)}>
                      Add to media bin
                    </InteriorButton>
                  </div>
                </article>
              ))}
            </div>
          </>
        );
      case "ai":
        return (
          <>
            <h2>Local AI assistant</h2>
            <p>Easy Edits Pro sends prompts only to Ollama running on this computer. Install Ollama and pull a model before use.</p>
            <label>
              Ollama model <input value={props.aiModel} maxLength={100} onChange={(event) => props.setAiModel(event.target.value)} />
            </label>
            <label>
              Prompt{" "}
              <textarea
                rows={5}
                maxLength={12000}
                value={props.aiPrompt}
                placeholder="Ask for editing ideas, a shot order, or help planning a sequence…"
                onChange={(event) => props.setAiPrompt(event.target.value)}
              />
            </label>
            <InteriorButton className="primary" disabled={props.busy || !props.aiPrompt.trim() || !props.aiModel.trim()} onClick={() => void props.askAi()}>
              Ask local model
            </InteriorButton>
            {props.aiReply && <pre className="ai-reply">{props.aiReply}</pre>}
            <small>AI suggestions do not modify the timeline or project automatically.</small>
          </>
        );
      default:
        if (props.selectedTransition) {
          return (
            <TransitionInspector
              selectedTransition={props.selectedTransition}
              setSelectedTransition={props.setSelectedTransition}
              updateTransition={props.updateTransition}
              removeSelectedTransition={props.removeSelectedTransition}
            />
          );
        }
        if (props.selectedTimelineClip && selectedClip) {
          const mediaForClip = props.media.find((item) => item.id === props.selectedTimelineClip!.media_id);
          if (mediaForClip?.kind === "title") {
            return (
              <TitleInspector
                selectedTimelineClip={props.selectedTimelineClip}
                mediaForClip={mediaForClip}
                titleDraft={props.titleDraft}
                setTitleDraft={props.setTitleDraft}
                saveTitleProperty={props.saveTitleProperty}
                removeSelectedClip={props.removeSelectedClip}
              />
            );
          }
        }
        return (
          <ClipInspector
            selectedTimelineClip={props.selectedTimelineClip}
            selectedClip={selectedClip}
            selectedMedia={props.selectedMedia}
            media={props.media}
            project={props.project}
            busy={props.busy}
            unlinkSelectedClips={props.unlinkSelectedClips}
            removeSelectedClip={props.removeSelectedClip}
            trackingMode={props.trackingMode}
            setTrackingMode={props.setTrackingMode}
            showTrackingBox={props.showTrackingBox}
            setShowTrackingBox={props.setShowTrackingBox}
            trackingSmoothness={props.trackingSmoothness}
            updateStabilizationSmoothness={props.updateStabilizationSmoothness}
            trackingTargetClipId={props.trackingTargetClipId}
            pinClipToMotionTrack={props.pinClipToMotionTrack}
            isAnalyzingTracking={props.isAnalyzingTracking}
            runMotionTracker={props.runMotionTracker}
            saveSpeedRamp={props.saveSpeedRamp}
            saveMotionBlur={props.saveMotionBlur}
            saveClipVolumeDb={props.saveClipVolumeDb}
            frameFit={props.frameFit}
            makeSubclip={props.makeSubclip}
            flattenMulticam={props.flattenMulticam}
            revealClipMedia={props.revealClipMedia}
            releaseNestedSequence={props.releaseNestedSequence}
          />
        );
    }
  }

  return (
    <aside className="editor-right-column">
      <StudioPanel title="Inspector" scrollable className="inspector-dock-panel">
        {renderInspectorContent()}
      </StudioPanel>
      <StudioPanel title="Audio Monitor &amp; Mixer" scrollable className="audio-monitor-mixer-panel">
        <AudioMonitorMixer
          project={props.project}
          masterVu={props.masterVu}
          vuLevels={props.vuLevels}
          loudnessReport={props.loudnessReport}
          masterGainDraft={props.masterGainDraft}
          setMasterGainDraft={props.setMasterGainDraft}
          gainDrafts={props.gainDrafts}
          setGainDrafts={props.setGainDrafts}
          saveTrackGain={props.saveTrackGain}
          toggleTrackMute={props.toggleTrackMute}
          toggleTrackSolo={props.toggleTrackSolo}
          audioClockMaster={props.audioClockMaster}
          activeClip={props.activeClip}
          media={props.media}
          playhead={props.playhead}
        />
      </StudioPanel>
    </aside>
  );
}
