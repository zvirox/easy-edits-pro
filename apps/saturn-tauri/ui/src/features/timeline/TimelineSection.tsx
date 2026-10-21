import { TimelineTrackLabel, TimelineTrackLane } from "./TimelineTrackLane";
import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction, PointerEvent } from "react";
import { useCallback } from "react";
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
  Sequence,
} from "../../app/projectTypes";
import type { CommandStatus } from "../../app/ProjectProvider";
import {
  type TimelineTool,
  timelineTools,
  TRACK_LABEL_WIDTH,
  TICKS_PER_SECOND,
  getTrackGaps,
} from "./types";
import { rulerSteps } from "./ruler";
import { TimelineToolbar, TimelineToolRail } from "./TimelineToolbar";

export {
  type TimelineTool,
  timelineTools,
  TRACK_LABEL_WIDTH,
  TICKS_PER_SECOND,
  getTrackGaps,
  rulerSteps,
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

  // New props for enhanced timeline
  sequences?: Sequence[];
  currentSequenceId?: number;
  addVideoTrack: () => Promise<void>;
  addAudioTrack: () => Promise<void>;
  onMediaDropToEmptySpace: (x: number, y: number, mediaId: number) => Promise<void>;
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
  // New props
  onMediaDropToEmptySpace,
}: TimelineSectionProps) {
  const tracks = project?.sequence.tracks ?? [];
  const videoTracks = tracks.filter((t) => t.kind === "video");
  const audioTracks = tracks.filter((t) => t.kind === "audio");
  const allSequences = project?.sequences ?? [];
  const hasSequences = allSequences.length > 1;
  const currentSequenceId = project?.sequence.id;

  // Calculate playhead position in pixels
  const playheadX = (playhead / TICKS_PER_SECOND) * timelineZoom;

  // Handler for dropping media on empty space (below/above tracks)
  const handleDropOnEmptySpace = useCallback(
    (e: React.DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      const raw = e.dataTransfer.getData("application/json");
      if (!raw) return;
      try {
        const parsed = JSON.parse(raw);
        if (parsed.media_id) {
          const rect = e.currentTarget.getBoundingClientRect();
          const x = e.clientX - rect.left;
          const y = e.clientY - rect.top;
          void onMediaDropToEmptySpace?.(x, y, parsed.media_id);
        }
      } catch {}
    },
    [onMediaDropToEmptySpace]
  );

  // Handler for dragging over empty space
  const handleDragOverEmptySpace = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }, []);

  return (
    <section className="timeline">
      {/* Timeline Header with Sequence Switcher */}
      <div className="timeline-header-bar">
        <TimelineToolRail
          timelineTool={timelineTool}
          setTimelineTool={setTimelineTool}
        />
        {hasSequences && (
          <div className="timeline-sequence-switcher">
            <label htmlFor="sequence-select" className="visually-hidden">
              Active Sequence
            </label>
            <select
              id="sequence-select"
              className="sequence-select"
              value={currentSequenceId ?? ""}
              onChange={(e) => void switchSequence(Number(e.target.value))}
              disabled={busy}
            >
              {allSequences.map((seq) => (
                <option key={seq.id ?? ""} value={seq.id ?? ""}>
                  {(seq.id ?? 0) === 1 ? "🎬 " : "⧉ "} {seq.name || `Sequence ${seq.id}`}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      <div className="timeline-workbench">
        <TimelineToolbar
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

        {/* Timeline Body (Labels + Content side by side) */}
        <div className="timeline-body">
          {/* Track Labels Area (fixed horizontal scroll, vertical scroll sync) */}
          <div className="timeline-labels-container">
          <div className="track-labels-header">
            <span className="track-spacer">Track</span>
          </div>
          <div
            className="track-labels-scroller"
            onPointerDown={handleTimelinePanStart}
            onPointerMove={handleTimelinePanMove}
            onPointerUp={handleTimelinePanEnd}
            onPointerCancel={handleTimelinePanEnd}
            onWheel={(e) => {
              // Sync horizontal scroll with timeline content
              if (e.deltaY !== 0) {
                const scroller = e.currentTarget;
                scroller.scrollLeft += e.deltaY;
              }
            }}
          >
            <div className="track-labels-content" style={{ minWidth: `${TRACK_LABEL_WIDTH}px` }}>
              {/* Subtitle Track Label */}
              <div className="track-label subtitle-track-label">
                <div className="track-label-top">
                  <b>Subtitles</b>
                  <span className="subtitle-badge-pill">CC</span>
                </div>
                <div className="track-label-btns subtitle-track-btns">
                  <InteriorButton
                    type="button"
                    className="track-mini-btn"
                    title="Add Subtitle at Playhead"
                    onClick={() => void addSubtitleAtPlayhead()}
                  >
                    ＋
                  </InteriorButton>
                  <InteriorButton
                    type="button"
                    className="track-mini-btn"
                    title="Import SRT/VTT Subtitles"
                    onClick={() => void importSubtitlesDialog()}
                  >
                    📥
                  </InteriorButton>
                  <InteriorButton
                    type="button"
                    className="track-mini-btn"
                    title="Export SRT/VTT Subtitles"
                    onClick={() => void exportSubtitlesDialog()}
                  >
                    📤
                  </InteriorButton>
                </div>
              </div>

              {/* Track Groups */}
              {(project?.sequence.track_groups ?? []).map((group) => {
                const memberTracks = tracks.filter((t) => t.group_id === group.id);
                return (
                  <div
                    className={`track-label track-group-label ${group.collapsed ? "is-collapsed" : ""}`}
                    key={`group-label-${group.id}`}
                    style={{
                      borderLeft: `4px solid ${group.color || "#9f1249"}`,
                    }}
                  >
                    <div className="track-group-title-row">
                      <InteriorButton
                        type="button"
                        className="track-group-collapse-btn"
                        onClick={() => void handleToggleGroupCollapse(group.id)}
                        title={group.collapsed ? "Expand Group" : "Collapse Group"}
                      >
                        {group.collapsed ? "▶" : "▼"}
                      </InteriorButton>
                      <span className="track-group-name" title={group.name}>
                        {group.name}
                      </span>
                      <small className="track-group-count">({memberTracks.length})</small>
                    </div>
                    <div className="track-group-actions">
                      <InteriorButton
                        type="button"
                        className="track-mini-btn"
                        onClick={() => void handleEditTrackGroup(group.id)}
                        title="Edit Folder Name / Color"
                      >
                        ✎
                      </InteriorButton>
                    </div>
                  </div>
                );
              })}

              {/* Video Tracks Labels */}
              {videoTracks.map((track) => {
                const parentGroup =
                  track.group_id != null
                    ? (project?.sequence.track_groups ?? []).find((g) => g.id === track.group_id)
                    : null;
                if (parentGroup?.collapsed) return null;
                return (
                  <TimelineTrackLabel
                    key={track.id}
                    track={track}
                    parentGroup={parentGroup}
                    toggleTrackLock={toggleTrackLock}
                    toggleTrackMute={toggleTrackMute}
                    toggleTrackSolo={toggleTrackSolo}
                    gainDrafts={gainDrafts}
                    setGainDrafts={setGainDrafts}
                    saveTrackGain={saveTrackGain}
                    panDrafts={panDrafts}
                    setPanDrafts={setPanDrafts}
                    saveTrackPan={saveTrackPan}
                    removeTrackFromTimeline={removeTrackFromTimeline}
                    busy={busy}
                    tracksCount={tracks.length}
                  />
                );
              })}

              {/* Audio Tracks Labels */}
              {audioTracks.map((track) => {
                const parentGroup =
                  track.group_id != null
                    ? (project?.sequence.track_groups ?? []).find((g) => g.id === track.group_id)
                    : null;
                if (parentGroup?.collapsed) return null;
                return (
                  <TimelineTrackLabel
                    key={track.id}
                    track={track}
                    parentGroup={parentGroup}
                    toggleTrackLock={toggleTrackLock}
                    toggleTrackMute={toggleTrackMute}
                    toggleTrackSolo={toggleTrackSolo}
                    gainDrafts={gainDrafts}
                    setGainDrafts={setGainDrafts}
                    saveTrackGain={saveTrackGain}
                    panDrafts={panDrafts}
                    setPanDrafts={setPanDrafts}
                    saveTrackPan={saveTrackPan}
                    removeTrackFromTimeline={removeTrackFromTimeline}
                    busy={busy}
                    tracksCount={tracks.length}
                  />
                );
              })}

              {/* Add Track Drop Zones */}
              <div
                className="track-label add-track-drop-zone video"
                onDragOver={handleDragOverEmptySpace}
                onDrop={handleDropOnEmptySpace}
                title="Drop video clip here to create new video track"
              >
                <span className="add-track-icon">＋</span>
                <span className="add-track-text">Add Video Track</span>
              </div>
              <div
                className="track-label add-track-drop-zone audio"
                onDragOver={handleDragOverEmptySpace}
                onDrop={handleDropOnEmptySpace}
                title="Drop audio clip here to create new audio track"
              >
                <span className="add-track-icon">＋</span>
                <span className="add-track-text">Add Audio Track</span>
              </div>
            </div>
          </div>
        </div>

        {/* Timeline Content Area (horizontal scroll) */}
        <div
          ref={attachTimelineScroller}
          className={`timeline-scroller tool-${timelineTool} ${timelineTool === "hand" ? "hand-mode" : ""}`}
          onPointerDown={handleTimelinePanStart}
          onPointerMove={handleTimelinePanMove}
          onPointerUp={handleTimelinePanEnd}
          onPointerCancel={handleTimelinePanEnd}
        >
          <div className="timeline-content" style={{ width: `${timelineWidth}px`, minWidth: `${timelineWidth}px` }}>
            {/* Single full-height playhead line (content-anchored so it spans
                ruler + all lanes on one continuous stroke). */}
            <i className="timeline-playhead-line" style={{ left: `${playheadX}px` }} />
            {/* FilmCraft-style ruler: adaptive labels + minor/major ticks.
                No spacer here — the ruler lane shares the track lanes' origin so
                the playhead, ruler labels and clips all line up on one x-axis. */}
            <div className="timeline-ruler">
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
                {(() => {
                  const { major: labelStep, minor: tickStep } = rulerSteps(timelineZoom);
                  const tickCount = Math.min(
                    Math.ceil(timelineWidth / (timelineZoom * tickStep)) + 1,
                    2000
                  );
                  const labelCount = Math.min(
                    Math.ceil(timelineWidth / (timelineZoom * labelStep)) + 1,
                    2000
                  );
                  return (
                    <>
                      {Array.from({ length: tickCount }, (_, index) => (
                        <i
                          key={`tick-${index}`}
                          className={`ruler-tick${(index * tickStep) % labelStep === 0 ? " major" : ""}`}
                          style={{ left: `${index * tickStep * timelineZoom}px` }}
                        />
                      ))}
                      {Array.from({ length: labelCount }, (_, index) => (
                        <span key={`label-${index}`} style={{ left: `${index * labelStep * timelineZoom}px` }}>
                          {formatTime(index * labelStep * TICKS_PER_SECOND)}
                        </span>
                      ))}
                    </>
                  );
                })()}
                {/* Unified Playhead - one grip in the ruler, its line spans the
                    whole timeline content (containing block: .timeline-content). */}
                <div
                  className="playhead-grip ruler-playhead-grip unified-playhead"
                  style={{ left: `${playheadX}px` }}
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

            {/* Subtitle Track Lane */}
            <div className="track-row subtitle-track-row">
              <div
                className="track-lane subtitle-track-lane"
                onClick={() => {
                  setSelectedSubtitle(null);
                  setSelectedClipId(null);
                  setSelectedClip(null);
                }}
                onDragOver={handleDragOverEmptySpace}
                onDrop={handleDropOnEmptySpace}
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

            {/* Track Groups Visual Folder Headers (Lanes) */}
            {(project?.sequence.track_groups ?? []).map((group) => {
              const memberTracks = tracks.filter((t) => t.group_id === group.id);
              return (
                <div
                  className={`track-row track-group-row ${group.collapsed ? "is-collapsed" : ""}`}
                  key={`group-lane-${group.id}`}
                >
                  <div
                    className="track-lane track-group-lane"
                    style={{
                      backgroundColor: `${group.color || "#9f1249"}0a`,
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

            {/* Video Tracks Lanes */}
            {videoTracks.map((track) => {
              const parentGroup =
                track.group_id != null
                  ? (project?.sequence.track_groups ?? []).find((g) => g.id === track.group_id)
                  : null;
              if (parentGroup?.collapsed) return null;
              return (
                <TimelineTrackLane
                  key={`video-${track.id}`}
                  track={track}
                  timelineTrackHeight={timelineTrackHeight}
                  timelineTool={timelineTool}
                  parentGroup={parentGroup}
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
                  mediaDropTargetTrackId={mediaDropTargetTrackId}
                  switchSequence={switchSequence}
                />
              );
            })}

            {/* Audio Tracks Lanes */}
            {audioTracks.map((track) => {
              const parentGroup =
                track.group_id != null
                  ? (project?.sequence.track_groups ?? []).find((g) => g.id === track.group_id)
                  : null;
              if (parentGroup?.collapsed) return null;
              return (
                <TimelineTrackLane
                  key={`audio-${track.id}`}
                  track={track}
                  timelineTrackHeight={timelineTrackHeight}
                  timelineTool={timelineTool}
                  parentGroup={parentGroup}
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
                  mediaDropTargetTrackId={mediaDropTargetTrackId}
                  switchSequence={switchSequence}
                />
              );
            })}

            {/* Empty Space Drop Zones - for auto track creation */}
            <div
              className="track-row empty-drop-zone video"
              onDragOver={handleDragOverEmptySpace}
              onDrop={handleDropOnEmptySpace}
            >
              <div className="track-lane empty-drop-lane video">
                <div className="empty-drop-hint">
                  <svg className="empty-drop-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <polygon points="23 7 16 12 23 17 23 7" />
                    <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
                  </svg>
                  <span>Drop video clip here to create new video track</span>
                </div>
              </div>
            </div>
            <div
              className="track-row empty-drop-zone audio"
              onDragOver={handleDragOverEmptySpace}
              onDrop={handleDropOnEmptySpace}
            >
              <div className="track-lane empty-drop-lane audio">
                <div className="empty-drop-hint">
                  <svg className="empty-drop-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M11 5L6 9H2v6h4l5 4V5z" />
                    <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07" />
                  </svg>
                  <span>Drop audio clip here to create new audio track</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
);
}

export { TimelineTrackLabel, TimelineTrackLane } from "./TimelineTrackLane";
