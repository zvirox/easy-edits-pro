import { useAutomationRecording } from "../../features/audio/useAutomationRecording";
import type { Track, AutomationInterpolation } from "../../types";
import type { AutomationLaneName } from "../../engine/playback/audioAutomation";

interface UseAutomationWritersArgs {
  perform: (command: string, payload: Record<string, unknown>) => Promise<unknown>;
  playing: boolean;
  playhead: number;
}

export function useAutomationWriters({ perform, playing, playhead }: UseAutomationWritersArgs) {
  // --- Automation recording (Phase 6 slice 1) -------------------------------
  const { recordAutomationWrite, commitAutomationWrites, hasAutomationWrites } =
    useAutomationRecording(perform);

  const automationWriteActive = (track: Track): boolean => {
    const mode = track.automation?.mode ?? "off";
    if (mode === "write") return true;
    return (mode === "latch" || mode === "touch") && playing;
  };

  const writeTrackAutomation = (track: Track, lane: AutomationLaneName, value: number) => {
    if (!automationWriteActive(track)) return;
    recordAutomationWrite(track.id, lane, playhead, value);
  };
  const commitTrackAutomation = (track: Track, lane: AutomationLaneName) => {
    if (!automationWriteActive(track)) return;
    if (!hasAutomationWrites(track.id, lane)) return;
    void commitAutomationWrites(track, lane);
  };
  const addAutomationLanePoint = async (
    trackId: number,
    lane: AutomationLaneName,
    time: number,
    value: number,
  ) => {
    await perform("track.add_automation_point", { track_id: trackId, lane, time, value });
  };
  const removeAutomationLanePoint = async (
    trackId: number,
    lane: AutomationLaneName,
    time: number,
  ) => {
    await perform("track.remove_automation_point", { track_id: trackId, lane, time });
  };
  const clearAutomationLane = async (trackId: number, lane: AutomationLaneName) => {
    await perform("track.clear_automation", { track_id: trackId, lane });
  };
  const setAutomationCurveInterpolation = async (
    trackId: number,
    interpolation: AutomationInterpolation,
  ) => {
    await perform("track.set_automation_interpolation", { track_id: trackId, interpolation });
  };

  return {
    writeTrackAutomation,
    commitTrackAutomation,
    addAutomationLanePoint,
    removeAutomationLanePoint,
    clearAutomationLane,
    setAutomationCurveInterpolation,
  };
}
