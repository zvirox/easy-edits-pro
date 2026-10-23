import { useCallback, useRef } from "react";
import type { AutomationPoint, Track } from "../../types";
import { recordLanePoint, type AutomationLaneName } from "../../engine/playback/audioAutomation";

type Perform = (id: string, params: Record<string, unknown>) => Promise<unknown>;

interface AutomationWrite {
  time: number;
  value: number;
}

/** Accumulates fader/pan writes during one automation-record gesture and
 * commits them to the track's automation lane in a single command (one undo
 * step). Touch-mode return-to-curve behaviour is a slice-1 simplification:
 * a touch drag records like a latch and keeps its written points on release. */
export function useAutomationRecording(perform: Perform) {
  const writesRef = useRef<Record<number, Partial<Record<AutomationLaneName, AutomationWrite[]>>>>({});

  const clearWrites = useCallback((trackId: number, lane: AutomationLaneName) => {
    const perTrack = writesRef.current[trackId];
    if (perTrack) delete perTrack[lane];
  }, []);

  /** Records one write at the playhead for the gesture in progress. */
  const recordAutomationWrite = useCallback(
    (trackId: number, lane: AutomationLaneName, playheadTicks: number, value: number) => {
      const perTrack = (writesRef.current[trackId] ??= {});
      const laneWrites = (perTrack[lane] ??= []);
      const last = laneWrites[laneWrites.length - 1];
      // Same playhead position (drag without playback motion): replace value.
      if (last && last.time === playheadTicks) {
        last.value = value;
        return;
      }
      laneWrites.push({ time: playheadTicks, value });
    },
    [],
  );

  /** True when the playhead moved at all since the first write (drags while
   * stopped produce a single point; playback drags produce the curve). */
  const hasAutomationWrites = useCallback((trackId: number, lane: AutomationLaneName) => {
    return (writesRef.current[trackId]?.[lane]?.length ?? 0) > 0;
  }, []);

  /** Merges the gesture's writes into the track's lane and commits once. */
  const commitAutomationWrites = useCallback(
    async (track: Track, lane: AutomationLaneName) => {
      const writes = writesRef.current[track.id]?.[lane] ?? [];
      if (!writes.length) return;
      writesRef.current[track.id] = { [lane]: [] };

      // Merge: existing curve points inside the written time range are
      // replaced by the recorded run (the fader took over that region).
      // Writes are collapsed with the same ~4 ms gap rule as the engine.
      let merged: AutomationPoint[] = track.automation?.[lane === "volume" ? "volume_points" : "pan_points"] ?? [];
      const bounds = writes.reduce(
        (range, write) => ({
          min: Math.min(range.min, write.time),
          max: Math.max(range.max, write.time),
        }),
        { min: Infinity, max: -Infinity },
      );
      merged = merged.filter((point) => point.time < bounds.min || point.time > bounds.max);
      for (const write of writes) {
        merged = recordLanePoint(merged, write.time, write.value);
      }

      const automation = {
        mode: track.automation?.mode ?? "off",
        interpolation: track.automation?.interpolation,
        volume_points: lane === "volume" ? merged : (track.automation?.volume_points ?? []),
        pan_points: lane === "pan" ? merged : (track.automation?.pan_points ?? []),
      };
      await perform(
        "track.set_automation",
        { track_id: track.id, automation },
      );
    },
    [perform],
  );

  return { recordAutomationWrite, commitAutomationWrites, clearWrites, hasAutomationWrites };
}
