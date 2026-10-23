import { useCallback, useEffect, useRef, useState } from "react";
import { audioClockMaster, evaluateTimelinePlayback, type TimelinePlaybackSnapshot } from "../engine/playbackEngine";
import { evaluateAutomationLane } from "../engine/playback/audioAutomation";
import type { MediaAsset, ProjectDocument, ProjectStatus } from "./projectTypes";

const TICKS_PER_SECOND = 254_016_000_000;

type Options = {
  projectState: ProjectStatus | null;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  preparedPreviews: Record<number, string>;
  mediaServerPort: number;
  mediaServerToken: string;
  clearProgramPreviewError: () => void;
};

export function usePlayback({ projectState, project, media, preparedPreviews, mediaServerPort, mediaServerToken, clearProgramPreviewError }: Options) {
  const [playhead, setPlayhead] = useState(0);
  const [programPlaying, setProgramPlaying] = useState(false);
  const [shuttleSpeed, setShuttleSpeed] = useState(1);
  const [deckSnapshot, setDeckSnapshot] = useState<TimelinePlaybackSnapshot | null>(null);
  const shuttleSpeedRef = useRef(1);
  const isPlayingRef = useRef(false);
  const clockStartAudioTimeRef = useRef(0);
  const clockStartPlayheadRef = useRef(0);
  const animFrameIdRef = useRef<number | null>(null);
  const playheadRef = useRef(0);
  const deckARef = useRef<HTMLVideoElement>(null);
  const deckBRef = useRef<HTMLVideoElement>(null);
  const audioPlayersRef = useRef<Map<string, HTMLAudioElement>>(new Map());
  const projectRef = useRef<ProjectDocument | null>(null);

  useEffect(() => { projectRef.current = projectState?.document ?? null; }, [projectState]);
  useEffect(() => { playheadRef.current = playhead; }, [playhead]);

  const syncDecks = useCallback((currentPlayhead: number, isPlaying: boolean) => {
    const doc = projectRef.current;
    if (!doc) return;
    const snapshot = evaluateTimelinePlayback(doc, currentPlayhead, media, preparedPreviews, mediaServerPort, mediaServerToken);
    setDeckSnapshot(snapshot);

    for (const [deck, state] of [[deckARef.current, snapshot.deckA], [deckBRef.current, snapshot.deckB]] as const) {
      if (!deck) continue;
      if (state.src) {
        if (deck.src !== state.src) {
          deck.src = state.src;
          try { deck.currentTime = state.currentTime; } catch { /* browser may not be seekable yet */ }
        } else if (Math.abs(deck.currentTime - state.currentTime) > 0.25) {
          try { deck.currentTime = state.currentTime; } catch { /* browser may not be seekable yet */ }
        }
        if (isPlaying && state.visible) {
          if (deck.paused) void deck.play().catch(() => {});
        } else if (!deck.paused) deck.pause();
      } else if (!deck.paused) deck.pause();
    }

    for (const audioItem of snapshot.activeAudioClips ?? []) {
      if (audioItem.track.muted) continue;
      const key = `audio-track-${audioItem.track.id}-${audioItem.clip.timeline_start}-${audioItem.clip.media_id}`;
      const element = audioPlayersRef.current.get(key);
      if (!element) continue;
      element.muted = false;
      const automation = audioItem.track.automation;
      const automationEnabled = !!automation && automation.mode !== "off";
      const gainDb = automationEnabled
        ? evaluateAutomationLane(automation.volume_points, currentPlayhead, audioItem.gainDb, automation.interpolation)
        : audioItem.gainDb;
      const pan = automationEnabled
        ? evaluateAutomationLane(automation.pan_points, currentPlayhead, audioItem.track.pan ?? 0, automation.interpolation)
        : audioItem.track.pan ?? 0;
      audioClockMaster.updateTrackGain(audioItem.track.id, gainDb);
      audioClockMaster.updateTrackPan(audioItem.track.id, pan);
      if (element.volume !== 1) element.volume = 1;
      if (Math.abs(element.currentTime - audioItem.currentTime) > 0.25) {
        try { element.currentTime = audioItem.currentTime; } catch { /* browser may not be seekable yet */ }
      }
      if (isPlaying) {
        if (element.paused) void element.play().catch(() => {});
      } else if (!element.paused) element.pause();
    }
  }, [media, preparedPreviews, mediaServerPort, mediaServerToken]);

  useEffect(() => {
    if (!isPlayingRef.current) syncDecks(playhead, false);
  }, [playhead, syncDecks]);

  const stopPlayback = useCallback((finalTicks?: number) => {
    if (animFrameIdRef.current) cancelAnimationFrame(animFrameIdRef.current);
    animFrameIdRef.current = null;
    isPlayingRef.current = false;
    setProgramPlaying(false);
    shuttleSpeedRef.current = 1;
    setShuttleSpeed(1);
    if (finalTicks !== undefined) {
      playheadRef.current = finalTicks;
      setPlayhead(finalTicks);
    }
    for (const deck of [deckARef.current, deckBRef.current]) if (deck && !deck.paused) deck.pause();
    audioPlayersRef.current.forEach((element) => { if (!element.paused) element.pause(); });
    syncDecks(playheadRef.current, false);
  }, [syncDecks]);

  const startPlayback = useCallback((speed = 1) => {
    const context = audioClockMaster.getContext();
    if (context?.state === "suspended") void context.resume();
    const sequenceDuration = Math.max(0, ...(project?.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.timeline_start + clip.duration)));
    if (playheadRef.current >= sequenceDuration && speed > 0) {
      playheadRef.current = project?.sequence.mark_in ?? 0;
      setPlayhead(playheadRef.current);
    }
    shuttleSpeedRef.current = speed;
    setShuttleSpeed(speed);
    isPlayingRef.current = true;
    setProgramPlaying(true);
    clearProgramPreviewError();
    clockStartAudioTimeRef.current = audioClockMaster.getCurrentTime();
    clockStartPlayheadRef.current = playheadRef.current;
    syncDecks(playheadRef.current, true);

    const tick = () => {
      if (!isPlayingRef.current) return;
      const elapsed = (audioClockMaster.getCurrentTime() - clockStartAudioTimeRef.current) * shuttleSpeedRef.current;
      const nextTicks = clockStartPlayheadRef.current + Math.round(elapsed * TICKS_PER_SECOND);
      const duration = Math.max(0, ...(projectRef.current?.project.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.timeline_start + clip.duration)));
      if (shuttleSpeedRef.current > 0 && nextTicks >= duration) { stopPlayback(duration); return; }
      if (shuttleSpeedRef.current < 0 && nextTicks <= 0) { stopPlayback(0); return; }
      playheadRef.current = nextTicks;
      setPlayhead(nextTicks);
      syncDecks(nextTicks, true);
      animFrameIdRef.current = requestAnimationFrame(tick);
    };
    if (animFrameIdRef.current) cancelAnimationFrame(animFrameIdRef.current);
    animFrameIdRef.current = requestAnimationFrame(tick);
  }, [project, syncDecks, stopPlayback, clearProgramPreviewError]);

  const toggleProgramPlayback = useCallback(() => {
    if (isPlayingRef.current) stopPlayback(); else startPlayback(1);
  }, [startPlayback, stopPlayback]);
  const seekProgramTo = useCallback((time: number) => {
    const target = Math.max(0, time);
    playheadRef.current = target;
    setPlayhead(target);
    if (isPlayingRef.current) {
      clockStartAudioTimeRef.current = audioClockMaster.getCurrentTime();
      clockStartPlayheadRef.current = target;
    }
    syncDecks(target, isPlayingRef.current);
  }, [syncDecks]);
  const stepProgramFrame = useCallback((direction: -1 | 1) => {
    if (isPlayingRef.current) stopPlayback();
    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
    seekProgramTo(playheadRef.current + direction * frameTicks);
  }, [project, seekProgramTo, stopPlayback]);
  const handleShuttleKey = useCallback((key: "j" | "k" | "l") => {
    if (key === "k") { stopPlayback(); return; }
    const direction = key === "l" ? 1 : -1;
    const speed = !isPlayingRef.current ? direction : shuttleSpeedRef.current === direction ? direction * 2 : shuttleSpeedRef.current === direction * 2 ? direction * 4 : direction;
    startPlayback(speed);
  }, [startPlayback, stopPlayback]);

  useEffect(() => () => {
    isPlayingRef.current = false;
    if (animFrameIdRef.current) cancelAnimationFrame(animFrameIdRef.current);
  }, []);

  return {
    playhead, setPlayhead, programPlaying, shuttleSpeed, deckSnapshot, deckARef, deckBRef,
    audioPlayersRef, playheadRef, isPlayingRef, syncDecks, stopPlayback, startPlayback,
    toggleProgramPlayback, seekProgramTo, stepProgramFrame, handleShuttleKey,
  };
}
