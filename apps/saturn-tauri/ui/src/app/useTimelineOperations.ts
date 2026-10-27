import {
  type PointerEvent,
  type SetStateAction,
  useCallback,
  useRef,
  useState,
} from "react";
import { getClipSelection, setMultiClips, setPrimaryClip, useClipSelection } from "./selectionStore";
import { expandLinkedClip, isLinkedSelection } from "./selectionModes";
import { useTrackGroupOperations } from "./useTrackGroupOperations";
import type {
  Clip,
  MediaAsset,
  ProjectDocument,
  SelectedClip,
  TimelineTool,
  Track,
  Transition,
} from "./projectTypes";
import { TICKS_PER_SECOND } from "../engine/playback/constants";
import type { KeyframeProperty } from "../engine/playback/types";
import { PANEL_OPEN_EVENT } from "../features/dock/panelEvents";
import { planTrim, type TrimEdge, type TrimSession } from "./timelineTrim.ts";
import { useTrimMode } from "./useTrimMode.ts";

type ClipPointerDrag = {
  pointerId: number;
  trackId: number;
  clipIndex: number;
  startX: number;
  startY: number;
  startTime: number;
  duration: number;
  tool: TimelineTool;
};

export type { TrimSession };

export type SpeedDurationApply = {
  speed?: number;
  new_duration?: number;
  linked: boolean;
};

type TimelineScrub = {
  pointerId: number;
  axis: HTMLElement;
};

export type UseTimelineOperationsOptions = {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  playhead: number;
  seekProgramTo: (ticks: number) => void;
  handleShuttleKey: (key: "j" | "k" | "l") => void;
  stopPlayback: () => void;
  timelineEndSeconds: number;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  setNotice: (msg: string) => void;
  busy: boolean;
  refreshProject: () => Promise<unknown>;
  setCompoundClipModalOpen: (open: boolean) => void;
  setCompoundClipNameDraft: (name: string) => void;
  keyframeProperty: KeyframeProperty;
  keyframeValue: string;
  initialSnapEnabled?: boolean;
  initialTrackHeight?: "compact" | "normal" | "expanded";
};

export function useTimelineOperations({
  project,
  media,
  playhead,
  seekProgramTo,
  handleShuttleKey,
  stopPlayback,
  timelineEndSeconds: _timelineEndSeconds,
  perform,
  runCommand,
  setNotice,
  busy: _busy,
  refreshProject,
  setCompoundClipModalOpen: _setCompoundClipModalOpen,
  setCompoundClipNameDraft: _setCompoundClipNameDraft,
  keyframeProperty: _keyframeProperty,
  keyframeValue: _keyframeValue,
  initialSnapEnabled = true,
  initialTrackHeight = "normal",
}: UseTimelineOperationsOptions) {
  const [timelineTool, setTimelineTool] = useState<TimelineTool>("select");
  const [timelineZoom, setTimelineZoom] = useState(72);
  const [snapEnabled, setSnapEnabled] = useState(initialSnapEnabled);
  const [timelineTrackHeight, setTimelineTrackHeight] = useState<"compact" | "normal" | "expanded">(initialTrackHeight);

  // F1: selection lives in one reactive store (app/selectionStore). The
  // setters keep the exact setState signatures the call sites already use;
  // the dead selectedClipId state is gone (it had no readers).
  const { clip: selectedClip, clips: selectedClips } = useClipSelection();
  const trackGroupOps = useTrackGroupOperations({ perform, project });
  const setSelectedClip = useCallback((clip: SetStateAction<SelectedClip | null>) => {
    const resolved = typeof clip === "function" ? clip(getClipSelection().clip) : clip;
    // Linked Selection: a click on a linked clip selects its whole link group.
    if (resolved && isLinkedSelection()) {
      const linked = expandLinkedClip(project ?? null, resolved);
      if (linked && linked.length > 1) setMultiClips(linked);
    }
    setPrimaryClip(resolved);
  }, [project]);
  const setSelectedClips = useCallback((clips: SetStateAction<SelectedClip[]>) => {
    setMultiClips(typeof clips === "function" ? clips(getClipSelection().clips) : clips);
  }, []);
  const [selectedTransition, setSelectedTransition] = useState<{ track_id: number; transition: Transition } | null>(null);

  const [gainDrafts, setGainDrafts] = useState<Record<number, number>>({});
  const [panDrafts, setPanDrafts] = useState<Record<number, number>>({});
  const [mediaDropTargetTrackId, setMediaDropTargetTrackId] = useState<number | null>(null);
  const [clipDragOffset, setClipDragOffset] = useState<{ trackId: number; clipIndex: number; x: number; y: number } | null>(null);

  const [trimSession, setTrimSession] = useState<TrimSession | null>(null);

  const clipDrag = useRef<ClipPointerDrag | null>(null);
  const suppressClipClick = useRef(false);
  const trimDrag = useRef<{
    pointerId: number;
    trackId: number;
    clipIndex: number;
    startX: number;
    edge: TrimEdge;
  } | null>(null);
  const gesturePending = useRef<Promise<unknown> | null>(null);

  const beginGesture = useCallback(() => {
    gesturePending.current = runCommand("edit.begin_gesture").catch(() => null);
  }, [runCommand]);

  const endGesture = useCallback(async () => {
    try {
      await gesturePending.current;
    } finally {
      gesturePending.current = null;
      await runCommand("edit.end_gesture").catch(() => null);
    }
  }, [runCommand]);

  /** Build the pure trim input from the project snapshot at gesture time. */
  const trimInput = useCallback(
    (trackId: number, clipIndex: number, edge: TrimEdge, deltaTicks: number) => {
      const track = project?.sequence.tracks.find((item) => item.id === trackId);
      const clip = track?.clips[clipIndex];
      if (!track || !clip) return null;
      return {
        trackId,
        tool: timelineTool,
        edge,
        deltaTicks,
        clipIndex,
        clip,
        previous: track.clips[clipIndex - 1] ?? null,
        next: track.clips[clipIndex + 1] ?? null,
        mediaDuration: media.find((item) => item.id === clip.media_id)?.duration ?? null,
      };
    },
    [project, media, timelineTool]
  );
  const handPan = useRef<{ startX: number; scrollLeft: number } | null>(null);
  const timelineScrollerRef = useRef<HTMLDivElement | null>(null);
  const activeTimelineScrub = useRef<TimelineScrub | null>(null);

  const zoomTimeline = useCallback((factor: number) => {
    setTimelineZoom((z) => Math.max(12, Math.min(320, Math.round(z * factor))));
  }, []);

  const attachTimelineScroller = useCallback((el: HTMLDivElement | null) => {
    timelineScrollerRef.current = el;
  }, []);

  const handleTimelinePanStart = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (timelineTool !== "hand") return;
      if (!timelineScrollerRef.current) return;
      event.preventDefault();
      handPan.current = {
        startX: event.clientX,
        scrollLeft: timelineScrollerRef.current.scrollLeft,
      };
    },
    [timelineTool]
  );

  const handleTimelinePanMove = useCallback((event: PointerEvent<HTMLDivElement>) => {
    if (!handPan.current || !timelineScrollerRef.current) return;
    const delta = event.clientX - handPan.current.startX;
    timelineScrollerRef.current.scrollLeft = handPan.current.scrollLeft - delta;
  }, []);

  const handleTimelinePanEnd = useCallback(() => {
    handPan.current = null;
  }, []);

  const saveTrackGain = useCallback(
    async (trackId: number, gainDb: number) => {
      await perform("track.set_gain_db", { track_id: trackId, gain_db: gainDb }, `Track ${trackId} gain set to ${gainDb} dB`);
    },
    [perform]
  );

  const saveTrackPan = useCallback(
    async (trackId: number, pan: number) => {
      await perform("track.set_pan", { track_id: trackId, pan }, `Track ${trackId} pan set to ${pan}`);
    },
    [perform]
  );

  const toggleTrackMute = useCallback(
    async (track: Track) => {
      await perform("track.set_muted", { track_id: track.id, muted: !track.muted }, `Track ${track.id} ${track.muted ? "unmuted" : "muted"}`);
    },
    [perform]
  );

  const toggleTrackSolo = useCallback(
    async (track: Track) => {
      await perform("track.set_solo", { track_id: track.id, solo: !track.solo }, `Track ${track.id} ${track.solo ? "un-soloed" : "soloed"}`);
    },
    [perform]
  );

  const toggleTrackLock = useCallback(
    async (track: Track) => {
      await perform("track.set_locked", { track_id: track.id, locked: !track.locked }, `Track ${track.id} ${track.locked ? "unlocked" : "locked"}`);
    },
    [perform]
  );

  const handleCreateTrackGroup = trackGroupOps.handleCreateTrackGroup;

  const switchSequence = useCallback(
    async (sequenceId: number) => {
      await perform("project.switch_sequence", { sequence_id: sequenceId }, "Switched active sequence");
      setSelectedClip(null);
      setSelectedClips([]);
      setSelectedTransition(null);
      seekProgramTo(0);
    },
    [perform, seekProgramTo]
  );

  const addTrackToTimeline = useCallback(
    async (kind: "video" | "audio") => {
      await perform("timeline.add_track", { kind }, `Added ${kind} track`);
    },
    [perform]
  );

  const removeTrackFromTimeline = useCallback(
    async (trackId: number) => {
      await perform("timeline.remove_track", { track_id: trackId }, "Removed track");
    },
    [perform]
  );

  const rippleCloseGap = useCallback(
    async (trackId: number, start: number, end: number) => {
      const gapDuration = end - start;
      await perform("timeline.ripple_delete_gap", { track_id: trackId, at_tick: start, duration: gapDuration }, "Ripple closed gap");
    },
    [perform]
  );

  const removeSelectedClip = useCallback(async () => {
    if (!selectedClip) return;
    await perform("timeline.remove_clip", selectedClip, "Clip removed");
    setSelectedClip(null);
    setSelectedClips([]);
  }, [selectedClip, perform]);

  const splitSelectedClip = useCallback(async () => {
    if (!selectedClip) return;
    await perform("timeline.split_clip", { ...selectedClip, at: playhead }, "Clip split at playhead");
  }, [selectedClip, playhead, perform]);

  const rippleDeleteSelectedClip = useCallback(async () => {
    if (!selectedClip) return;
    await perform("timeline.ripple_delete", selectedClip, "Clip ripple deleted");
    setSelectedClip(null);
    setSelectedClips([]);
  }, [selectedClip, perform]);

  const applySpeedDuration = useCallback(
    async (patch: SpeedDurationApply) => {
      if (!selectedClip) {
        setNotice("Select a timeline clip first");
        return false;
      }
      return perform(
        "timeline.rate_stretch",
        { ...selectedClip, ...patch },
        "Clip speed and duration updated"
      );
    },
    [selectedClip, perform, setNotice]
  );

  const linkSelectedClips = useCallback(async () => {
    if (selectedClips.length < 2) return;
    const clipCoords = selectedClips.map((c) => [c.track_id, c.clip_index]);
    await perform("timeline.link_clips", { clips: clipCoords }, "Linked selected clips");
  }, [selectedClips, perform]);

  const unlinkSelectedClips = useCallback(async () => {
    if (selectedClip) {
      await perform("timeline.unlink_clips", { track_id: selectedClip.track_id, clip_index: selectedClip.clip_index }, "Unlinked clip");
    }
  }, [selectedClip, perform]);

  const createCompoundClip = useCallback(
    async (customName?: string) => {
      const clipsToCompound = selectedClips.length > 0 ? selectedClips : selectedClip ? [selectedClip] : [];
      if (clipsToCompound.length === 0) return;
      const clipCoords = clipsToCompound.map((c) => [c.track_id, c.clip_index]);
      await perform("timeline.create_compound_clip", { clips: clipCoords, name: customName || "Compound Clip" }, "Created compound clip sequence");
      setSelectedClips([]);
      setSelectedClip(null);
    },
    [selectedClips, selectedClip, perform]
  );

  const decomposeSelectedCompoundClip = useCallback(async () => {
    if (!selectedClip) return;
    await perform("timeline.decompose_compound_clip", { track_id: selectedClip.track_id, clip_index: selectedClip.clip_index }, "Decomposed compound clip");
  }, [selectedClip, perform]);

  /** Source-monitor 3-point edit: send a marked range to the playhead. */
  const threePointEdit = useCallback(
    async (
      mode: "insert" | "overwrite",
      mediaId: number,
      sourceInTicks: number,
      durationTicks: number,
      atTicks: number,
    ) => {
      const asset = media.find((item) => item.id === mediaId);
      if (!asset) {
        setNotice("That source clip is no longer in the media pool");
        return;
      }
      const kind = asset.kind === "audio" ? "audio" : "video";
      const track = project?.sequence.tracks.find(
        (item) => item.kind === kind && !item.locked,
      );
      if (!track) {
        setNotice(`Add an unlocked ${kind} track before a three-point edit`);
        return;
      }
      const label = mode === "insert" ? "Inserted" : "Overwrote";
      await perform(
        mode === "insert" ? "timeline.insert_clip" : "timeline.overwrite_clip",
        {
          media_id: mediaId,
          track_id: track.id,
          at: Math.max(0, atTicks),
          source_in: Math.max(0, sourceInTicks),
          duration: Math.max(1, durationTicks),
        },
        `${label} ${asset.name} at the playhead`,
      );
    },
    [media, project, perform, setNotice],
  );

  const beginTimelineScrub = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      event.stopPropagation();
      const axis = event.currentTarget;
      axis.setPointerCapture(event.pointerId);
      activeTimelineScrub.current = { pointerId: event.pointerId, axis };
      const bounds = axis.getBoundingClientRect();
      const raw = Math.max(0, ((event.clientX - bounds.left) / timelineZoom) * TICKS_PER_SECOND);
      seekProgramTo(raw);
    },
    [timelineZoom, seekProgramTo]
  );

  const moveTimelineScrub = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (!activeTimelineScrub.current || activeTimelineScrub.current.pointerId !== event.pointerId) return;
      const bounds = activeTimelineScrub.current.axis.getBoundingClientRect();
      const raw = Math.max(0, ((event.clientX - bounds.left) / timelineZoom) * TICKS_PER_SECOND);
      seekProgramTo(raw);
    },
    [timelineZoom, seekProgramTo]
  );

  const finishTimelineScrub = useCallback((event: PointerEvent<HTMLDivElement>) => {
    if (activeTimelineScrub.current && activeTimelineScrub.current.pointerId === event.pointerId) {
      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {}
      activeTimelineScrub.current = null;
    }
  }, []);

  const beginTimelineClipPointer = useCallback(
    (event: PointerEvent<HTMLDivElement>, trackId: number, clipIndex: number, clip: Clip) => {
      const track = project?.sequence.tracks.find((t) => t.id === trackId);
      if (!track) return;
      if (track.locked) {
        setNotice(`Track "${track.name}" is locked`);
        return;
      }
      if (timelineTool === "razor") {
        event.preventDefault();
        event.stopPropagation();
        const bounds = event.currentTarget.getBoundingClientRect();
        const ratio = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
        const at = Math.round(clip.timeline_start + clip.duration * ratio);
        void perform(
          "timeline.split_clip",
          { track_id: trackId, clip_index: clipIndex, at },
          "Clip split at pointer"
        );
        return;
      }
      if (timelineTool === "trackForward" || timelineTool === "trackBackward") {
        event.preventDefault();
        event.stopPropagation();
        const forward = timelineTool === "trackForward";
        const selection = track.clips
          .map((entry, entryIndex) => ({ entry, entryIndex }))
          .filter(({ entry }) =>
            forward
              ? entry.timeline_start >= clip.timeline_start
              : entry.timeline_start <= clip.timeline_start
          )
          .map(({ entryIndex }) => ({ track_id: trackId, clip_index: entryIndex }));
        setSelectedClips(selection);
        setSelectedClip({ track_id: trackId, clip_index: clipIndex });
        setSelectedTransition(null);
        setNotice(
          `${selection.length} clip${selection.length === 1 ? "" : "s"} selected on ${track.name}`
        );
        return;
      }
      if (
        timelineTool === "select" ||
        timelineTool === "slip" ||
        timelineTool === "slide" ||
        timelineTool === "rateStretch"
      ) {
        event.preventDefault();
        event.stopPropagation();
        event.currentTarget.setPointerCapture(event.pointerId);
        clipDrag.current = {
          pointerId: event.pointerId,
          trackId,
          clipIndex,
          startX: event.clientX,
          startY: event.clientY,
          startTime: clip.timeline_start,
          duration: clip.duration,
          tool: timelineTool,
        };
        setSelectedClip({ track_id: trackId, clip_index: clipIndex });
        setSelectedTransition(null);
        if (timelineTool !== "select") {
          beginGesture();
          setTrimSession({ trackId, clipIndex, tool: timelineTool, edge: "end", deltaTicks: 0 });
          window.setTimeout(() => {
            window.dispatchEvent(
              new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: "trim-monitor" } }),
            );
          }, 0);
        }
        return;
      }
      if (timelineTool === "ripple" || timelineTool === "rolling") {
        event.preventDefault();
        event.stopPropagation();
        setSelectedClip({ track_id: trackId, clip_index: clipIndex });
        setSelectedTransition(null);
        setNotice(`Drag a clip edge with the ${timelineTool === "ripple" ? "Ripple" : "Rolling"} tool`);
      }
    },
    [project, timelineTool, setNotice, perform, beginGesture]
  );

  const commitTrim = useCallback(
    async (trackId: number, clipIndex: number, edge: TrimEdge, deltaTicks: number) => {
      const input = trimInput(trackId, clipIndex, edge, deltaTicks);
      if (!input) {
        setNotice("That clip is no longer available");
        await endGesture();
        setTrimSession(null);
        return;
      }
      const result = planTrim(input);
      if (!result.ok) {
        setNotice(result.reason);
        await endGesture();
        setTrimSession(null);
        return;
      }
      try {
        await gesturePending.current;
        for (const command of result.plan.commands) {
          await runCommand(command.id, command.params);
        }
        await refreshProject();
        setNotice(result.plan.notice);
      } catch (error) {
        setNotice(String(error));
      } finally {
        await endGesture();
        setTrimSession(null);
      }
    },
    [trimInput, endGesture, runCommand, refreshProject, setNotice]
  );

  const trimModeOps = useTrimMode({
    selectedClip,
    project,
    timelineTool,
    playhead,
    beginGesture,
    endGesture,
    seekProgramTo,
    handleShuttleKey,
    stopPlayback,
    setNotice,
    trimSession,
    setTrimSession,
    trimInput,
    commitTrim,
  });

  const moveTimelineClipPointer = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (!clipDrag.current || clipDrag.current.pointerId !== event.pointerId) return;
      const deltaX = event.clientX - clipDrag.current.startX;
      const deltaY = event.clientY - clipDrag.current.startY;
      if (Math.abs(deltaX) > 4 || Math.abs(deltaY) > 4) {
        suppressClipClick.current = true;
      }
      if (clipDrag.current.tool === "select") {
        setClipDragOffset({
          trackId: clipDrag.current.trackId,
          clipIndex: clipDrag.current.clipIndex,
          x: deltaX,
          y: deltaY,
        });
        return;
      }
      const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
      const input = trimInput(
        clipDrag.current.trackId,
        clipDrag.current.clipIndex,
        "end",
        deltaTicks,
      );
      if (!input) return;
      const result = planTrim(input);
      if (!result.ok) {
        setNotice(result.reason);
        return;
      }
      setTrimSession({
        trackId: clipDrag.current.trackId,
        clipIndex: clipDrag.current.clipIndex,
        tool: clipDrag.current.tool,
        edge: "end",
        deltaTicks: result.plan.deltaTicks,
      });
      setNotice(result.plan.notice);
    },
    [timelineZoom, trimInput, setNotice]
  );

  const handleTimelineClipPointerCancel = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (!clipDrag.current || clipDrag.current.pointerId !== event.pointerId) return;
      const drag = clipDrag.current;
      clipDrag.current = null;
      setClipDragOffset(null);
      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {}
      const deltaX = event.clientX - drag.startX;
      if (drag.tool !== "select") {
        if (Math.abs(deltaX) <= 4) {
          void endGesture();
          setTrimSession(null);
          return;
        }
        const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
        void commitTrim(drag.trackId, drag.clipIndex, "end", deltaTicks);
        return;
      }
      if (Math.abs(deltaX) > 4) {
        const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
        // Multi-selection drag moves all selected clips together (keeping
        // relative offsets) via the batch command; single drag stays atomic.
        const draggedInSelection = selectedClips.some(
          (c) => c.track_id === drag.trackId && c.clip_index === drag.clipIndex
        );
        if (draggedInSelection && selectedClips.length > 1) {
          const tracks = project?.sequence.tracks ?? [];
          const placed = selectedClips
            .map((ref) => {
              const clip = tracks.find((t) => t.id === ref.track_id)?.clips[ref.clip_index];
              return clip ? { ref, start: clip.timeline_start } : null;
            })
            .filter((x): x is { ref: SelectedClip; start: number } => x !== null);
          if (placed.length > 1) {
            void perform(
              "timeline.move_clips_batch",
              {
                moves: placed.map(({ ref, start }) => ({
                  track_id: ref.track_id,
                  clip_index: ref.clip_index,
                  timeline_start: Math.max(0, start + deltaTicks),
                })),
              },
              `Moved ${placed.length} clips`
            );
            return;
          }
        }
        const newStart = Math.max(0, drag.startTime + deltaTicks);
        void perform(
          "timeline.move_clip",
          {
            track_id: drag.trackId,
            clip_index: drag.clipIndex,
            target_track_id: drag.trackId,
            timeline_start: newStart,
          },
          "Moved clip"
        );
      }
    },
    [timelineZoom, perform, commitTrim, endGesture, project, selectedClips]
  );

  const beginTimelineClipTrim = useCallback(
    (
      event: PointerEvent<HTMLSpanElement>,
      trackId: number,
      clipIndex: number,
      _clip: Clip,
      edge: TrimEdge
    ) => {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      trimDrag.current = {
        pointerId: event.pointerId,
        trackId,
        clipIndex,
        startX: event.clientX,
        edge,
      };
      setSelectedClip({ track_id: trackId, clip_index: clipIndex });
      setSelectedTransition(null);
      beginGesture();
      setTrimSession({ trackId, clipIndex, tool: timelineTool, edge, deltaTicks: 0 });
      window.setTimeout(() => {
        window.dispatchEvent(
          new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: "trim-monitor" } }),
        );
      }, 0);
    },
    [timelineTool, beginGesture]
  );

  const moveTimelineClipTrim = useCallback(
    (event: PointerEvent<HTMLSpanElement>) => {
      if (!trimDrag.current || trimDrag.current.pointerId !== event.pointerId) return;
      const deltaX = event.clientX - trimDrag.current.startX;
      const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
      const input = trimInput(
        trimDrag.current.trackId,
        trimDrag.current.clipIndex,
        trimDrag.current.edge,
        deltaTicks,
      );
      if (!input) return;
      const result = planTrim(input);
      if (!result.ok) {
        setNotice(result.reason);
        return;
      }
      setTrimSession({
        trackId: trimDrag.current.trackId,
        clipIndex: trimDrag.current.clipIndex,
        tool: timelineTool,
        edge: trimDrag.current.edge,
        deltaTicks: result.plan.deltaTicks,
      });
      setNotice(result.plan.notice);
    },
    [timelineZoom, trimInput, timelineTool, setNotice]
  );

  const finishTimelineClipTrim = useCallback(
    (event: PointerEvent<HTMLSpanElement>) => {
      if (!trimDrag.current || trimDrag.current.pointerId !== event.pointerId) return;
      const drag = trimDrag.current;
      trimDrag.current = null;
      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {}
      const deltaX = event.clientX - drag.startX;
      if (Math.abs(deltaX) <= 2) {
        void endGesture();
        setTrimSession(null);
        return;
      }
      const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
      void commitTrim(drag.trackId, drag.clipIndex, drag.edge, deltaTicks);
    },
    [timelineZoom, commitTrim, endGesture]
  );

  return {
    timelineTool,
    setTimelineTool,
    timelineZoom,
    setTimelineZoom,
    snapEnabled,
    setSnapEnabled,
    timelineTrackHeight,
    setTimelineTrackHeight,
    selectedClip,
    setSelectedClip,
    selectedClips,
    setSelectedClips,
    selectedTransition,
    setSelectedTransition,
    gainDrafts,
    setGainDrafts,
    panDrafts,
    setPanDrafts,
    mediaDropTargetTrackId,
    setMediaDropTargetTrackId,
    clipDragOffset,
    setClipDragOffset,
    trimSession,
    setTrimSession,
    ...trimModeOps,
    zoomTimeline,
    attachTimelineScroller,
    handleTimelinePanStart,
    handleTimelinePanMove,
    handleTimelinePanEnd,
    saveTrackGain,
    saveTrackPan,
    toggleTrackMute,
    toggleTrackSolo,
    toggleTrackLock,
    handleCreateTrackGroup,
    handleToggleGroupCollapse: trackGroupOps.handleToggleGroupCollapse,
    handleEditTrackGroup: trackGroupOps.handleEditTrackGroup,
    handleDeleteTrackGroup: trackGroupOps.handleDeleteTrackGroup,
    handleAssignTrackToGroup: trackGroupOps.handleAssignTrackToGroup,
    switchSequence,
    addTrackToTimeline,
    removeTrackFromTimeline,
    rippleCloseGap,
    removeSelectedClip,
    splitSelectedClip,
    rippleDeleteSelectedClip,
    applySpeedDuration,
    linkSelectedClips,
    unlinkSelectedClips,
    createCompoundClip,
    decomposeSelectedCompoundClip,
    threePointEdit,
    beginTimelineScrub,
    moveTimelineScrub,
    finishTimelineScrub,
    beginTimelineClipPointer,
    moveTimelineClipPointer,
    handleTimelineClipPointerCancel,
    beginTimelineClipTrim,
    moveTimelineClipTrim,
    finishTimelineClipTrim,
  };
}
