import type { Dispatch, SetStateAction, PointerEvent } from "react";
import { Icon } from "../../components/Icon";
import { formatTime } from "../../utils/formatters";
import type { AudioClockMaster } from "../../engine/playbackEngine";
import type {
  Clip,
  MediaAsset,
  ProjectDocument,
  SubtitleItem,
  Track,
  Transition,
  SelectedClip,
} from "../../app/projectTypes";
import type { CommandStatus } from "../../app/ProjectProvider";
import {
  type TimelineTool,
  timelineTools,
  TRACK_LABEL_WIDTH,
  TICKS_PER_SECOND,
  getTrackGaps,
} from "./types";
import { TimelineToolbar } from "./TimelineToolbar";
import { TimelineTrackRow } from "./TimelineTrackRow";

export {
  type TimelineTool,
  timelineTools,
  TRACK_LABEL_WIDTH,
  TICKS_PER_SECOND,
  getTrackGaps,
};

export interface TimelineSectionProps {
  timelineTool: TimelineTool;
  setTimelineTool: (tool: TimelineTool) => void;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  switchSequence: (seqId: number) => Promise<void>;
  playhead: number;
  busy: boolean;
  addTrackToTimeline: (kind: "video" | "audio") => Promise<void>;
  timelineTrackHeight: "compact" | "normal" | "expanded";
  setTimelineTrackHeight: Dispatch<SetStateAction<"compact" | "normal" | "expanded">>;
  snapEnabled: boolean;
  setSnapEnabled: Dispatch<SetStateAction<boolean>>;
  addSelectedToTimeline: () => Promise<void>;
  selectedMedia: MediaAsset | null;
  addTitleToTimeline: () => Promise<void>;
  addTransitionToTimeline: () => Promise<void>;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  perform: (id: string, params?: unknown) => Promise<unknown>;
  selectedClip: SelectedClip | null;
  splitSelectedClip: () => Promise<void>;
  rippleDeleteSelectedClip: () => Promise<void>;
  removeSelectedClip: () => void;
  selectedClips: SelectedClip[];
  linkSelectedClips: () => Promise<void>;
  unlinkSelectedClips: () => Promise<void>;
  setCompoundClipNameDraft: (name: string) => void;
  setCompoundClipModalOpen: (open: boolean) => void;
  decomposeSelectedCompoundClip: () => Promise<void>;
  invokeMenuCommand: (cmd: string) => Promise<void>;
  zoomTimeline: (factor: number) => void;
  handleCreateTrackGroup: () => Promise<void>;

  attachTimelineScroller: (el: HTMLDivElement | null) => void;
  handleTimelinePanStart: (e: PointerEvent<HTMLDivElement>) => void;
  handleTimelinePanMove: (e: PointerEvent<HTMLDivElement>) => void;
  handleTimelinePanEnd: () => void;
  timelineWidth: number;
  timelineZoom: number;
  beginTimelineScrub: (e: PointerEvent<HTMLDivElement>) => void;
  moveTimelineScrub: (e: PointerEvent<HTMLDivElement>) => void;
  finishTimelineScrub: (e: PointerEvent<HTMLDivElement>) => void;
  timelineEndSeconds: number;

  addSubtitleAtPlayhead: () => Promise<void>;
  importSubtitlesDialog: () => Promise<void>;
  exportSubtitlesDialog: () => Promise<void>;
  selectedSubtitle: SubtitleItem | null;
  setSelectedSubtitle: Dispatch<SetStateAction<SubtitleItem | null>>;
  setSelectedClipId: Dispatch<SetStateAction<number | null>>;
  setSelectedClip: Dispatch<SetStateAction<SelectedClip | null>>;

  handleToggleGroupCollapse: (groupId: number) => Promise<void>;
  handleEditTrackGroup: (groupId: number) => Promise<void>;
  toggleTrackLock: (track: Track) => Promise<void>;
  toggleTrackMute: (track: Track) => Promise<void>;
  toggleTrackSolo: (track: Track) => Promise<void>;
  gainDrafts: Record<number, number>;
  setGainDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackGain: (trackId: number, val: number) => Promise<void>;
  audioClockMaster: AudioClockMaster;
  panDrafts: Record<number, number>;
  setPanDrafts: Dispatch<SetStateAction<Record<number, number>>>;
  saveTrackPan: (trackId: number, val: number) => Promise<void>;
  removeTrackFromTimeline: (trackId: number) => Promise<void>;
  mediaDropTargetTrackId: number | null;
  clipDragOffset: any;
  rippleCloseGap: (trackId: number, start: number, end: number) => void;
  selectedTransition: { track_id: number; transition: Transition } | null;
  setSelectedTransition: Dispatch<SetStateAction<{ track_id: number; transition: Transition } | null>>;
  missingMediaIds: Set<number>;
  beginTimelineClipPointer: (e: PointerEvent<HTMLDivElement>, trackId: number, clipIdx: number, clip: Clip) => void;
  moveTimelineClipPointer: (e: PointerEvent<HTMLDivElement>) => void;
  handleTimelineClipPointerCancel: (e: PointerEvent<HTMLDivElement>) => void;
  beginTimelineClipTrim: (e: PointerEvent<HTMLSpanElement>, trackId: number, clipIdx: number, clip: Clip, handle: "start" | "end") => void;
  moveTimelineClipTrim: (e: PointerEvent<HTMLSpanElement>) => void;
  finishTimelineClipTrim: (e: PointerEvent<HTMLSpanElement>) => void;
}

export function TimelineSection({
  timelineTool,
  setTimelineTool,
  project,
  media,
  switchSequence,
  playhead,
  busy,
  addTrackToTimeline,
  timelineTrackHeight,
  setTimelineTrackHeight,
  snapEnabled,
  setSnapEnabled,
  addSelectedToTimeline,
  selectedMedia,
  addTitleToTimeline,
  addTransitionToTimeline,
  undoStatus,
  redoStatus,
  perform,
  selectedClip,
  splitSelectedClip,
  rippleDeleteSelectedClip,
  removeSelectedClip,
  selectedClips,
  linkSelectedClips,
  unlinkSelectedClips,
  setCompoundClipNameDraft,
  setCompoundClipModalOpen,
  decomposeSelectedCompoundClip,
  invokeMenuCommand,
  zoomTimeline,
  handleCreateTrackGroup,
  attachTimelineScroller,
  handleTimelinePanStart,
  handleTimelinePanMove,
  handleTimelinePanEnd,
  timelineWidth,
  timelineZoom,
  beginTimelineScrub,
  moveTimelineScrub,
  finishTimelineScrub,
  timelineEndSeconds,
  addSubtitleAtPlayhead,
  importSubtitlesDialog,
  exportSubtitlesDialog,
  selectedSubtitle,
  setSelectedSubtitle,
  setSelectedClipId,
  setSelectedClip,
  handleToggleGroupCollapse,
  handleEditTrackGroup,
  toggleTrackLock,
  toggleTrackMute,
  toggleTrackSolo,
  gainDrafts,
  setGainDrafts,
  saveTrackGain,
  audioClockMaster,
  panDrafts,
  setPanDrafts,
  saveTrackPan,
  removeTrackFromTimeline,
  mediaDropTargetTrackId,
  clipDragOffset,
  rippleCloseGap,
  selectedTransition,
  setSelectedTransition,
  missingMediaIds,
  beginTimelineClipPointer,
  moveTimelineClipPointer,
  handleTimelineClipPointerCancel,
  beginTimelineClipTrim,
  moveTimelineClipTrim,
  finishTimelineClipTrim,
}: TimelineSectionProps) {
  const tracks = project?.sequence.tracks ?? [];

  return (
    <section className="timeline">
      <TimelineToolbar
        timelineTool={timelineTool}
        setTimelineTool={setTimelineTool}
        project={project}
        switchSequence={switchSequence}
        playhead={playhead}
        busy={busy}
        addTrackToTimeline={addTrackToTimeline}
        timelineTrackHeight={timelineTrackHeight}
        setTimelineTrackHeight={setTimelineTrackHeight}
        snapEnabled={snapEnabled}
        setSnapEnabled={setSnapEnabled}
        addSelectedToTimeline={addSelectedToTimeline}
        selectedMedia={selectedMedia}
        addTitleToTimeline={addTitleToTimeline}
        addTransitionToTimeline={addTransitionToTimeline}
        undoStatus={undoStatus}
        redoStatus={redoStatus}
        perform={perform}
        selectedClip={selectedClip}
        splitSelectedClip={splitSelectedClip}
        rippleDeleteSelectedClip={rippleDeleteSelectedClip}
        removeSelectedClip={removeSelectedClip}
        selectedClips={selectedClips}
        linkSelectedClips={linkSelectedClips}
        unlinkSelectedClips={unlinkSelectedClips}
        setCompoundClipNameDraft={setCompoundClipNameDraft}
        setCompoundClipModalOpen={setCompoundClipModalOpen}
        decomposeSelectedCompoundClip={decomposeSelectedCompoundClip}
        invokeMenuCommand={invokeMenuCommand}
        zoomTimeline={zoomTimeline}
        handleCreateTrackGroup={handleCreateTrackGroup}
      />
      <div className="timeline-workbench">
        <div
          ref={attachTimelineScroller}
          className={`timeline-scroller ${timelineTool === "hand" ? "hand-mode" : ""}`}
          onPointerDown={handleTimelinePanStart}
          onPointerMove={handleTimelinePanMove}
          onPointerUp={handleTimelinePanEnd}
          onPointerCancel={handleTimelinePanEnd}
        >
          <div className="timeline-content" style={{ width: `${timelineWidth}px`, minWidth: `${timelineWidth}px` }}>
            <div className="timeline-ruler">
              <span className="track-spacer">Time</span>
              <div
                className="ruler-lane"
                onPointerDown={beginTimelineScrub}
                onPointerMove={moveTimelineScrub}
                onPointerUp={finishTimelineScrub}
                onPointerCancel={finishTimelineScrub}
              >
                {project?.sequence.mark_in != null && (
                  <i
                    className="sequence-marker in-point"
                    style={{ left: `${(project.sequence.mark_in / TICKS_PER_SECOND) * timelineZoom}px` }}
                    title={`In · ${formatTime(project.sequence.mark_in)}`}
                  >
                    <Icon name="markIn" />
                  </i>
                )}
                {project?.sequence.mark_out != null && (
                  <i
                    className="sequence-marker out-point"
                    style={{ left: `${(project.sequence.mark_out / TICKS_PER_SECOND) * timelineZoom}px` }}
                    title={`Out · ${formatTime(project.sequence.mark_out)}`}
                  >
                    <Icon name="markOut" />
                  </i>
                )}
                {Array.from({ length: Math.ceil(timelineWidth / (timelineZoom * 5)) + 1 }, (_, index) => (
                  <span key={index} style={{ left: `${index * timelineZoom * 5}px` }}>
                    {formatTime(index * TICKS_PER_SECOND * 5)}
                  </span>
                ))}
                <div
                  className="playhead-grip ruler-playhead-grip"
                  style={{ left: `${(playhead / TICKS_PER_SECOND) * timelineZoom}px` }}
                  role="slider"
                  aria-label="Timeline playhead"
                  aria-valuenow={playhead / TICKS_PER_SECOND}
                  aria-valuemin={0}
                  aria-valuemax={timelineEndSeconds}
                  onPointerDown={beginTimelineScrub}
                  onPointerMove={moveTimelineScrub}
                  onPointerUp={finishTimelineScrub}
                  onPointerCancel={finishTimelineScrub}
                >
                  <i className="playhead" />
                </div>
              </div>
            </div>

            {/* Dedicated Subtitle Track Lane */}
            <div className="track-row subtitle-track-row">
              <div className="track-label subtitle-track-label">
                <div className="track-label-top">
                  <b>Subtitles</b>
                  <span className="subtitle-badge-pill">CC</span>
                </div>
                <div className="track-label-btns subtitle-track-btns">
                  <button
                    type="button"
                    className="track-mini-btn"
                    title="Add Subtitle at Playhead"
                    onClick={() => void addSubtitleAtPlayhead()}
                  >
                    ＋
                  </button>
                  <button
                    type="button"
                    className="track-mini-btn"
                    title="Import SRT/VTT Subtitles"
                    onClick={() => void importSubtitlesDialog()}
                  >
                    📥
                  </button>
                  <button
                    type="button"
                    className="track-mini-btn"
                    title="Export SRT/VTT Subtitles"
                    onClick={() => void exportSubtitlesDialog()}
                  >
                    📤
                  </button>
                </div>
              </div>
              <div
                className="track-lane subtitle-track-lane"
                onClick={() => {
                  setSelectedSubtitle(null);
                  setSelectedClipId(null);
                  setSelectedClip(null);
                }}
              >
                {(project?.sequence.subtitles ?? []).map((sub) => {
                  const left = (sub.start_ticks / TICKS_PER_SECOND) * timelineZoom;
                  const width = Math.max(34, ((sub.end_ticks - sub.start_ticks) / TICKS_PER_SECOND) * timelineZoom);
                  const isSelected = selectedSubtitle?.id === sub.id;
                  return (
                    <div
                      key={sub.id}
                      className={`timeline-clip subtitle-clip ${isSelected ? "selected" : ""}`}
                      style={{ left: `${left}px`, width: `${width}px` }}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedSubtitle(sub);
                        setSelectedClipId(null);
                        setSelectedClip(null);
                      }}
                      title={`Subtitle #${sub.id}: "${sub.text}" (${formatTime(sub.start_ticks)} - ${formatTime(sub.end_ticks)})`}
                    >
                      <span className="subtitle-clip-tag">CC</span>
                      <span className="subtitle-clip-label">{sub.text || "Empty caption"}</span>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Track Groups Visual Folder Headers */}
            {(project?.sequence.track_groups ?? []).map((group) => {
              const memberTracks = tracks.filter((t) => t.group_id === group.id);
              return (
                <div
                  className={`track-row track-group-row ${group.collapsed ? "is-collapsed" : ""}`}
                  key={`group-${group.id}`}
                >
                  <div
                    className="track-label track-group-label"
                    style={{
                      borderLeft: `4px solid ${group.color || "#3182ce"}`,
                    }}
                  >
                    <div className="track-group-title-row">
                      <button
                        type="button"
                        className="track-group-collapse-btn"
                        onClick={() => void handleToggleGroupCollapse(group.id)}
                        title={group.collapsed ? "Expand Group" : "Collapse Group"}
                      >
                        {group.collapsed ? "▶" : "▼"}
                      </button>
                      <span className="track-group-name" title={group.name}>
                        {group.name}
                      </span>
                      <small className="track-group-count">({memberTracks.length})</small>
                    </div>
                    <div className="track-group-actions">
                      <button
                        type="button"
                        className="track-mini-btn"
                        onClick={() => void handleEditTrackGroup(group.id)}
                        title="Edit Folder Name / Color"
                      >
                        ✎
                      </button>
                    </div>
                  </div>
                  <div
                    className="track-lane track-group-lane"
                    style={{
                      backgroundColor: `${group.color || "#3182ce"}0a`,
                    }}
                  >
                    {group.collapsed && (
                      <span className="track-group-collapsed-hint">
                        📁 {group.name} ({memberTracks.length} tracks hidden) — Click arrow to expand
                      </span>
                    )}
                  </div>
                </div>
              );
            })}

            {tracks.map((track) => {
              const parentGroup =
                track.group_id != null
                  ? (project?.sequence.track_groups ?? []).find((g) => g.id === track.group_id)
                  : null;
              if (parentGroup?.collapsed) {
                return null;
              }
              return (
                <TimelineTrackRow
                  key={track.id}
                  track={track}
                  timelineTrackHeight={timelineTrackHeight}
                  parentGroup={parentGroup}
                  toggleTrackLock={toggleTrackLock}
                  toggleTrackMute={toggleTrackMute}
                  toggleTrackSolo={toggleTrackSolo}
                  gainDrafts={gainDrafts}
                  setGainDrafts={setGainDrafts}
                  audioClockMaster={audioClockMaster}
                  saveTrackGain={saveTrackGain}
                  panDrafts={panDrafts}
                  setPanDrafts={setPanDrafts}
                  saveTrackPan={saveTrackPan}
                  removeTrackFromTimeline={removeTrackFromTimeline}
                  busy={busy}
                  tracksCount={tracks.length}
                  mediaDropTargetTrackId={mediaDropTargetTrackId}
                  timelineZoom={timelineZoom}
                  perform={perform}
                  rippleCloseGap={rippleCloseGap}
                  selectedTransition={selectedTransition}
                  setSelectedTransition={setSelectedTransition}
                  setSelectedClipId={setSelectedClipId}
                  setSelectedClip={setSelectedClip}
                  media={media}
                  missingMediaIds={missingMediaIds}
                  selectedClip={selectedClip}
                  selectedClips={selectedClips}
                  clipDragOffset={clipDragOffset}
                  beginTimelineClipPointer={beginTimelineClipPointer}
                  moveTimelineClipPointer={moveTimelineClipPointer}
                  handleTimelineClipPointerCancel={handleTimelineClipPointerCancel}
                  beginTimelineClipTrim={beginTimelineClipTrim}
                  moveTimelineClipTrim={moveTimelineClipTrim}
                  finishTimelineClipTrim={finishTimelineClipTrim}
                  switchSequence={switchSequence}
                  playhead={playhead}
                  timelineEndSeconds={timelineEndSeconds}
                  beginTimelineScrub={beginTimelineScrub}
                  moveTimelineScrub={moveTimelineScrub}
                  finishTimelineScrub={finishTimelineScrub}
                />
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}
