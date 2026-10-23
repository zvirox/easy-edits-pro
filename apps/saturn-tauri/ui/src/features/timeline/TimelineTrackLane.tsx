import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, MouseEvent, SetStateAction, PointerEvent } from "react";
import { formatTime, getClipWaveform } from "../../utils/formatters";
import type {
  Clip,
  MediaAsset,
  Track,
  Transition,
  SelectedClip,
} from "../../app/projectTypes";
import {
  TICKS_PER_SECOND,
  getTrackGaps,
  type TimelineTool,
} from "./types";


// Separate Track Label Component for better organization
interface TimelineTrackLabelProps {
  track: Track;
  parentGroup?: { id: number; name: string; color?: string; collapsed?: boolean } | null;
  toggleTrackLock: (track: Track) => Promise<void>;
  toggleTrackMute: (track: Track) => Promise<void>;
  toggleTrackSolo: (track: Track) => Promise<void>;
  removeTrackFromTimeline: (trackId: number) => Promise<void>;
  busy: boolean;
  tracksCount: number;
}

export function TimelineTrackLabel({
  track,
  parentGroup,
  toggleTrackLock,
  toggleTrackMute,
  toggleTrackSolo,
  removeTrackFromTimeline,
  busy,
  tracksCount,
}: TimelineTrackLabelProps) {
  return (
    <div
      className={`track-label ${track.locked ? "is-locked" : ""} ${track.muted ? "is-muted" : ""} ${parentGroup ? "is-grouped-track" : ""}`}
      style={parentGroup ? { borderLeft: `3px solid ${parentGroup.color || "#9f1249"}` } : undefined}
    >
      <div className="track-label-top">
        <div style={{ display: "flex", alignItems: "center", gap: "4px", minWidth: 0 }}>
          <span className={`track-kind-badge ${track.kind}`}>{track.kind === "audio" ? "A" : "V"}</span>
          <b>{track.name}</b>
          {track.audio_bus && track.audio_bus !== "Master" && (
            <span className="track-bus-pill" title={`Routing Bus: ${track.audio_bus}`}>
              {track.audio_bus}
            </span>
          )}
        </div>
      </div>
      {/* G4 compact header (44px): badge + name above, track state below.
          Gain/pan faders stay in the Inspector and the Fairlight mixer —
          the header carries no sliders (plan v4 directive). */}
      <div className="track-label-btns">
        <InteriorButton
          type="button"
          className={`track-mini-btn ${track.locked ? "active lock" : ""}`}
          onClick={() => void toggleTrackLock(track)}
          title={track.locked ? "Unlock track" : "Lock track"}
        >
          {track.locked ? "🔒" : "🔓"}
        </InteriorButton>
        <InteriorButton
          type="button"
          className={`track-mini-btn ${track.muted ? "active mute" : ""}`}
          onClick={() => void toggleTrackMute(track)}
          title={track.muted ? "Unmute track" : "Mute track"}
        >
          M
        </InteriorButton>
        <InteriorButton
          type="button"
          className={`track-mini-btn ${track.solo ? "active solo" : ""}`}
          onClick={() => void toggleTrackSolo(track)}
          title={track.solo ? "Solo active" : "Solo track"}
        >
          S
        </InteriorButton>
        {track.linked_group_id != null && (
          <span className="track-group-pill" title={`Audio/Video Sync Group #${track.linked_group_id}`}>
            🔗
          </span>
        )}
        <span style={{ flex: 1 }} />
        <InteriorButton
          className="track-delete-btn"
          onClick={() => void removeTrackFromTimeline(track.id)}
          disabled={busy || tracksCount <= 1}
          title="Delete Track"
        >
          ×
        </InteriorButton>
      </div>
    </div>
  );
}

// Separate Track Lane Component for better organization
interface TimelineTrackLaneProps {
  track: Track;
  timelineTrackHeight: "compact" | "normal" | "expanded";
  timelineTool: TimelineTool;
  parentGroup?: { id: number; name: string; color?: string; collapsed?: boolean } | null;
  timelineZoom: number;
  perform: (id: string, params?: unknown) => Promise<unknown>;
  rippleCloseGap: (trackId: number, start: number, end: number) => void;
  selectedTransition: { track_id: number; transition: Transition } | null;
  setSelectedTransition: Dispatch<SetStateAction<{ track_id: number; transition: Transition } | null>>;
  setSelectedClip: Dispatch<SetStateAction<SelectedClip | null>>;
  media: MediaAsset[];
  missingMediaIds: Set<number>;
  selectedClip: SelectedClip | null;
  selectedClips: SelectedClip[];
  clipDragOffset: any;
  beginTimelineClipPointer: (e: PointerEvent<HTMLDivElement>, trackId: number, clipIdx: number, clip: Clip) => void;
  moveTimelineClipPointer: (e: PointerEvent<HTMLDivElement>) => void;
  handleTimelineClipPointerCancel: (e: PointerEvent<HTMLDivElement>) => void;
  beginTimelineClipTrim: (e: PointerEvent<HTMLSpanElement>, trackId: number, clipIdx: number, clip: Clip, handle: "start" | "end") => void;
  moveTimelineClipTrim: (e: PointerEvent<HTMLSpanElement>) => void;
  finishTimelineClipTrim: (e: PointerEvent<HTMLSpanElement>) => void;
  mediaDropTargetTrackId: number | null;
  switchSequence: (seqId: number) => Promise<void>;
  onClipContextMenu: (
    event: MouseEvent<HTMLDivElement>,
    trackId: number,
    clipIndex: number,
  ) => void;
}

export function TimelineTrackLane({
  track,
  timelineTrackHeight,
  timelineTool,
  parentGroup,
  timelineZoom,
  perform,
  rippleCloseGap,
  selectedTransition,
  setSelectedTransition,
  setSelectedClip,
  media,
  missingMediaIds,
  selectedClip,
  selectedClips,
  clipDragOffset,
  beginTimelineClipPointer,
  moveTimelineClipPointer,
  handleTimelineClipPointerCancel,
  beginTimelineClipTrim,
  moveTimelineClipTrim,
  finishTimelineClipTrim,
  mediaDropTargetTrackId,
  switchSequence,
  onClipContextMenu,
}: TimelineTrackLaneProps) {
  return (
    <div
      className={`track-row track-height-${timelineTrackHeight} ${track.locked ? "is-locked" : ""} ${track.muted ? "is-muted" : ""} ${parentGroup ? "is-grouped-track" : ""}`}
      style={parentGroup ? { borderLeft: `3px solid ${parentGroup.color || "#9f1249"}` } : undefined}
    >
      <div
        className={`track-lane ${mediaDropTargetTrackId === track.id ? "media-drop-target" : ""}`}
        data-track-id={track.id}
        onDragOver={(e) => {
          e.preventDefault();
          e.dataTransfer.dropEffect = "copy";
        }}
        onDrop={(e) => {
          e.preventDefault();
          const raw = e.dataTransfer.getData("application/json");
          if (!raw) return;
          try {
            const parsed = JSON.parse(raw);
            if (parsed.media_id) {
              const rect = e.currentTarget.getBoundingClientRect();
              const clickX = e.clientX - rect.left;
              const startTicks = Math.max(0, Math.round((clickX / timelineZoom) * TICKS_PER_SECOND));
              void perform("timeline.add_clip", {
                track_id: track.id,
                media_id: parsed.media_id,
                timeline_start: startTicks,
              });
            }
          } catch {}
        }}
      >
        {/* Empty Gap Detection & 1-Click Ripple Deletion */}
        {getTrackGaps(track.clips).map((gap, gIdx) => {
          const gapLeft = (gap.start / TICKS_PER_SECOND) * timelineZoom;
          const gapWidth = ((gap.end - gap.start) / TICKS_PER_SECOND) * timelineZoom;
          if (gapWidth < 6) return null;
          return (
            <div
              key={`gap-${track.id}-${gap.start}-${gIdx}`}
              className="timeline-gap-segment"
              style={{ left: `${gapLeft}px`, width: `${gapWidth}px` }}
              title={`Empty Gap: ${((gap.end - gap.start) / TICKS_PER_SECOND).toFixed(2)}s · Click to ripple close`}
              onClick={(e) => {
                e.stopPropagation();
                void rippleCloseGap(track.id, gap.start, gap.end);
              }}
            >
              <span className="gap-close-icon">×</span>
            </div>
          );
        })}

        {/* Track Transitions Badges */}
        {(track.transitions ?? []).map((trans) => {
          const cutX = (trans.cut_at / TICKS_PER_SECOND) * timelineZoom;
          const durPx = Math.max(26, (trans.duration / TICKS_PER_SECOND) * timelineZoom);
          const left = Math.max(0, cutX - durPx / 2);
          const isSelected =
            selectedTransition?.track_id === track.id && selectedTransition?.transition.id === trans.id;
          const label = trans.kind.replace(/_/g, " ");
          return (
            <div
              key={`trans-${track.id}-${trans.id}`}
              className={`timeline-transition-badge ${isSelected ? "selected" : ""}`}
              style={{ left: `${left}px`, width: `${durPx}px` }}
              onClick={(e) => {
                e.stopPropagation();
                setSelectedTransition({ track_id: track.id, transition: trans });
                setSelectedClip(null);
              }}
              title={`Transition: ${label} (${(trans.duration / TICKS_PER_SECOND).toFixed(2)}s)`}
            >
              <span className="trans-icon">⧓</span>
              <span className="trans-label">{label}</span>
            </div>
          );
        })}

        {track.clips.map((clip, index) => {
          const item = media.find((entry) => entry.id === clip.media_id);
          const isMissing = item && (missingMediaIds.has(item.id) || item.is_missing);
          const isSelected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
          const isMultiSelected = selectedClips.some(
            (sc) => sc.track_id === track.id && sc.clip_index === index
          );
          const isPointerDragging =
            clipDragOffset &&
            clipDragOffset.trackId === track.id &&
            clipDragOffset.clipIndex === index;
          const isCompound = clip.compound_sequence_id != null || item?.kind === "compound_sequence";
          const isAudioLike = track.kind === "audio" || item?.kind === "audio";

          const clipLeft = (clip.timeline_start / TICKS_PER_SECOND) * timelineZoom;
          const clipWidth = Math.max(24, (clip.duration / TICKS_PER_SECOND) * timelineZoom);

          return (
            <div
              className={`timeline-clip tool-${timelineTool} ${isAudioLike ? "audio" : "video"} ${isSelected || isMultiSelected ? "selected" : ""} ${isMissing ? "is-missing" : ""} ${isPointerDragging ? "pointer-dragging" : ""} ${isCompound ? "is-compound-clip" : ""}`}
              style={{
                left: `${clipLeft}px`,
                width: `${clipWidth}px`,
                borderLeftColor: item?.color_tag || undefined,
              }}
              key={`${clip.media_id}-${clip.timeline_start}-${index}`}
              data-media-id={clip.media_id}
              data-source-in={clip.source_in}
              data-timeline-start={clip.timeline_start}
              onPointerDown={(event) => beginTimelineClipPointer(event, track.id, index, clip)}
              onPointerMove={moveTimelineClipPointer}
              onPointerUp={handleTimelineClipPointerCancel}
              onPointerCancel={handleTimelineClipPointerCancel}
              onContextMenu={(event) =>
                onClipContextMenu(event, track.id, index)
              }
            >
              {/* Left Trim Handle */}
              <span
                className="clip-trim-handle clip-trim-left"
                onPointerDown={(e) => beginTimelineClipTrim(e, track.id, index, clip, "start")}
                onPointerMove={moveTimelineClipTrim}
                onPointerUp={finishTimelineClipTrim}
                onPointerCancel={finishTimelineClipTrim}
                title="Drag to trim Clip In Point"
              />

              {/* Right Trim Handle */}
              <span
                className="clip-trim-handle clip-trim-right"
                onPointerDown={(e) => beginTimelineClipTrim(e, track.id, index, clip, "end")}
                onPointerMove={moveTimelineClipTrim}
                onPointerUp={finishTimelineClipTrim}
                onPointerCancel={finishTimelineClipTrim}
                title="Drag to trim Clip Out Point"
              />

              {/* Audio Waveform visualization */}
              {(() => {
                const gradId = `wf-grad-edit-${track.id}-${clip.media_id}-${index}`;
                const isAudio = isAudioLike;
                const activeColor = isAudio ? "#34d399" : "#be185d";
                const inactiveColor = isAudio ? "rgba(52, 211, 153, 0.42)" : "rgba(159, 18, 73, 0.42)";

                return (
                  <div className="clip-waveform-container" aria-hidden="true">
                    <svg className="clip-waveform-svg" viewBox="0 0 100 32" preserveAspectRatio="none">
                      <defs>
                        <linearGradient id={gradId} x1="0%" y1="0%" x2="100%" y2="0%">
                          <stop offset="0%" stopColor={activeColor} />
                          <stop offset="100%" stopColor={inactiveColor} />
                        </linearGradient>
                      </defs>
                      <path
                        d={getClipWaveform(item)
                          .map((val, i, arr) => {
                            const h = Math.max(1, (val / 255) * 14);
                            const x = (i / arr.length) * 100;
                            return `M ${x.toFixed(1)} ${(16 - h).toFixed(1)} L ${x.toFixed(1)} ${(16 + h).toFixed(1)}`;
                          })
                          .join(" ")}
                        stroke={`url(#${gradId})`}
                        strokeWidth="1.2"
                        strokeLinecap="round"
                      />
                    </svg>
                  </div>
                );
              })()}

              <strong>{item?.name ?? `Media #${clip.media_id}`}</strong>
              <small>
                {isMissing
                  ? "⚠ Offline Media"
                  : `${formatTime(clip.source_in)} · ${formatTime(clip.duration)}`}
              </small>
              {isCompound && (
                <span
                  className="clip-compound-badge"
                  title="Compound Clip · Double-click to open nested sequence"
                  onClick={(e) => {
                    e.stopPropagation();
                    const targetSeqId = clip.compound_sequence_id ?? item?.nested_sequence?.id;
                    if (targetSeqId) void switchSequence(targetSeqId);
                  }}
                >
                  ⧉ Compound
                </span>
              )}
              {clip.speed && clip.speed !== 1.0 && (
                <span className="clip-speed-badge">⚡{Math.round(clip.speed * 100)}%</span>
              )}
              {clip.group_id != null && (
                <span className="clip-group-badge" title={`Linked Group #${clip.group_id}`}>
                  🔗
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
