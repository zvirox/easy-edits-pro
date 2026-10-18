import {
  type PointerEvent,
  useCallback,
  useRef,
  useState,
} from "react";
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

type ClipPointerDrag = {
  pointerId: number;
  trackId: number;
  clipIndex: number;
  startX: number;
  startY: number;
  startTime: number;
  duration: number;
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
  timelineEndSeconds: number;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  setNotice: (msg: string) => void;
  busy: boolean;
  refreshProject: () => Promise<unknown>;
  setCompoundClipModalOpen: (open: boolean) => void;
  setCompoundClipNameDraft: (name: string) => void;
  keyframeProperty: KeyframeProperty;
  keyframeValue: string;
};

export function useTimelineOperations({
  project,
  media: _media,
  playhead,
  seekProgramTo,
  timelineEndSeconds: _timelineEndSeconds,
  perform,
  setNotice,
  busy: _busy,
  refreshProject: _refreshProject,
  setCompoundClipModalOpen: _setCompoundClipModalOpen,
  setCompoundClipNameDraft: _setCompoundClipNameDraft,
  keyframeProperty: _keyframeProperty,
  keyframeValue: _keyframeValue,
}: UseTimelineOperationsOptions) {
  const [timelineTool, setTimelineTool] = useState<TimelineTool>("select");
  const [timelineZoom, setTimelineZoom] = useState(72);
  const [snapEnabled, setSnapEnabled] = useState(true);
  const [timelineTrackHeight, setTimelineTrackHeight] = useState<"compact" | "normal" | "expanded">("normal");

  const [selectedClip, setSelectedClip] = useState<SelectedClip | null>(null);
  const [selectedClips, setSelectedClips] = useState<SelectedClip[]>([]);
  const [selectedClipId, setSelectedClipId] = useState<number | null>(null);
  const [selectedTransition, setSelectedTransition] = useState<{ track_id: number; transition: Transition } | null>(null);

  const [gainDrafts, setGainDrafts] = useState<Record<number, number>>({});
  const [panDrafts, setPanDrafts] = useState<Record<number, number>>({});
  const [mediaDropTargetTrackId, setMediaDropTargetTrackId] = useState<number | null>(null);
  const [clipDragOffset, setClipDragOffset] = useState<{ trackId: number; clipIndex: number; x: number; y: number } | null>(null);

  const clipDrag = useRef<ClipPointerDrag | null>(null);
  const suppressClipClick = useRef(false);
  const trimDrag = useRef<{
    pointerId: number;
    trackId: number;
    clipIndex: number;
    startX: number;
    handle: "start" | "end";
    initialStart: number;
    initialDur: number;
    sourceIn: number;
  } | null>(null);
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
      await perform("track.set_gain", { track_id: trackId, gain_db: gainDb }, `Track ${trackId} gain set to ${gainDb} dB`);
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
      await perform("track.set_mute", { track_id: track.id, muted: !track.muted }, `Track ${track.id} ${track.muted ? "unmuted" : "muted"}`);
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
      await perform("track.set_lock", { track_id: track.id, locked: !track.locked }, `Track ${track.id} ${track.locked ? "unlocked" : "locked"}`);
    },
    [perform]
  );

  const handleCreateTrackGroup = useCallback(async () => {
    const name = window.prompt("Enter new folder group name:", "Audio Stems");
    if (!name?.trim()) return;
    const colors = ["#38bdf8", "#10b981", "#f59e0b", "#ec4899", "#8b5cf6", "#06b6d4"];
    const color = colors[Math.floor(Math.random() * colors.length)];
    await perform("track_group.create", { name: name.trim(), color }, `Folder group "${name}" created`);
  }, [perform]);

  const handleToggleGroupCollapse = useCallback(
    async (groupId: number) => {
      await perform("track_group.toggle_collapse", { group_id: groupId });
    },
    [perform]
  );

  const handleEditTrackGroup = useCallback(
    async (groupId: number) => {
      const group = (project?.sequence.track_groups ?? []).find((g) => g.id === groupId);
      if (!group) return;
      const newName = window.prompt("Rename track folder:", group.name);
      if (!newName?.trim()) return;
      await perform("track_group.set_properties", { group_id: groupId, name: newName.trim(), color: group.color }, `Renamed folder to "${newName}"`);
    },
    [perform, project]
  );

  const switchSequence = useCallback(
    async (sequenceId: number) => {
      await perform("sequence.switch", { sequence_id: sequenceId }, "Switched active sequence");
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
      if (track?.locked) {
        setNotice(`Track "${track.name}" is locked`);
        return;
      }
      if (timelineTool === "select" || timelineTool === "ripple") {
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
        };
        setSelectedClip({ track_id: trackId, clip_index: clipIndex });
        setSelectedTransition(null);
      }
    },
    [project, timelineTool, setNotice]
  );

  const moveTimelineClipPointer = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (!clipDrag.current || clipDrag.current.pointerId !== event.pointerId) return;
      const deltaX = event.clientX - clipDrag.current.startX;
      const deltaY = event.clientY - clipDrag.current.startY;
      if (Math.abs(deltaX) > 4 || Math.abs(deltaY) > 4) {
        suppressClipClick.current = true;
      }
      setClipDragOffset({
        trackId: clipDrag.current.trackId,
        clipIndex: clipDrag.current.clipIndex,
        x: deltaX,
        y: deltaY,
      });
    },
    []
  );

  const handleTimelineClipPointerCancel = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (clipDrag.current && clipDrag.current.pointerId === event.pointerId) {
        const drag = clipDrag.current;
        clipDrag.current = null;
        setClipDragOffset(null);
        try {
          event.currentTarget.releasePointerCapture(event.pointerId);
        } catch {}
        const deltaX = event.clientX - drag.startX;
        if (Math.abs(deltaX) > 4) {
          const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
          const newStart = Math.max(0, drag.startTime + deltaTicks);
          void perform(
            "timeline.move_clip",
            {
              from_track: drag.trackId,
              from_index: drag.clipIndex,
              to_track: drag.trackId,
              to_start: newStart,
            },
            "Moved clip"
          );
        }
      }
    },
    [timelineZoom, perform]
  );

  const beginTimelineClipTrim = useCallback(
    (
      event: PointerEvent<HTMLSpanElement>,
      trackId: number,
      clipIndex: number,
      clip: Clip,
      handle: "start" | "end"
    ) => {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      trimDrag.current = {
        pointerId: event.pointerId,
        trackId,
        clipIndex,
        startX: event.clientX,
        handle,
        initialStart: clip.timeline_start,
        initialDur: clip.duration,
        sourceIn: clip.source_in,
      };
      setSelectedClip({ track_id: trackId, clip_index: clipIndex });
      setSelectedTransition(null);
    },
    []
  );

  const moveTimelineClipTrim = useCallback(
    (event: PointerEvent<HTMLSpanElement>) => {
      if (!trimDrag.current || trimDrag.current.pointerId !== event.pointerId) return;
      const deltaX = event.clientX - trimDrag.current.startX;
      const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
      if (trimDrag.current.handle === "start") {
        const newDur = Math.max(TICKS_PER_SECOND / 30, trimDrag.current.initialDur - deltaTicks);
        setNotice(`Trimming Start: ${(newDur / TICKS_PER_SECOND).toFixed(2)}s`);
      } else {
        const newDur = Math.max(TICKS_PER_SECOND / 30, trimDrag.current.initialDur + deltaTicks);
        setNotice(`Trimming End: ${(newDur / TICKS_PER_SECOND).toFixed(2)}s`);
      }
    },
    [timelineZoom, setNotice]
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
      const deltaTicks = Math.round((deltaX / timelineZoom) * TICKS_PER_SECOND);
      if (drag.handle === "start") {
        const proposedStart = Math.max(0, drag.initialStart + deltaTicks);
        const proposedDur = Math.max(TICKS_PER_SECOND / 30, drag.initialDur - deltaTicks);
        const proposedIn = Math.max(0, drag.sourceIn + deltaTicks);
        void perform(
          "timeline.trim_clip",
          {
            track_id: drag.trackId,
            clip_index: drag.clipIndex,
            timeline_start: proposedStart,
            source_in: proposedIn,
            duration: proposedDur,
          },
          "Trimmed clip head"
        );
      } else {
        const proposedDur = Math.max(TICKS_PER_SECOND / 30, drag.initialDur + deltaTicks);
        void perform(
          "timeline.trim_clip",
          {
            track_id: drag.trackId,
            clip_index: drag.clipIndex,
            timeline_start: drag.initialStart,
            source_in: drag.sourceIn,
            duration: proposedDur,
          },
          "Trimmed clip tail"
        );
      }
    },
    [timelineZoom, perform]
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
    selectedClipId,
    setSelectedClipId,
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
    handleToggleGroupCollapse,
    handleEditTrackGroup,
    switchSequence,
    addTrackToTimeline,
    removeTrackFromTimeline,
    rippleCloseGap,
    removeSelectedClip,
    splitSelectedClip,
    rippleDeleteSelectedClip,
    linkSelectedClips,
    unlinkSelectedClips,
    createCompoundClip,
    decomposeSelectedCompoundClip,
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
