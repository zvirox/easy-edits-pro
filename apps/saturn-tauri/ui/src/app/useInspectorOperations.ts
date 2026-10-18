import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import type {
  Clip,
  ColorAdjustments,
  ProjectDocument,
  SubtitleItem,
} from "./projectTypes";
import type { SelectedClip } from "../features/menubar";
import type {
  EffectKind,
  EffectPayload,
  InterpolationKind,
  KeyframeProperty,
  MotionBlurParams,
  SpeedRamp,
  TrackingMode,
} from "../engine/playbackEngine";
import { type ColorWheelsData, DEFAULT_COLOR_WHEELS } from "../components/ColorWheels";
import { TICKS_PER_SECOND } from "../features/timeline/types";

export interface UseInspectorOperationsOptions {
  selectedClip: SelectedClip | null;
  selectedTimelineClip: Clip | null;
  playhead: number;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  refreshProject: () => Promise<unknown>;
  setNotice: (msg: string) => void;
  setBusy: (busy: boolean) => void;
  project: ProjectDocument["project"] | null;
}

export function useInspectorOperations({
  selectedClip,
  selectedTimelineClip,
  playhead,
  perform,
  runCommand,
  refreshProject,
  setNotice,
  setBusy,
  project,
}: UseInspectorOperationsOptions) {
  const [colorDraft, setColorDraft] = useState<ColorAdjustments>({ exposure: 0, contrast: 1, saturation: 1, hue: 0 });
  const [colorWheelsDraft, setColorWheelsDraft] = useState<ColorWheelsData>(DEFAULT_COLOR_WHEELS);

  const [keyframeProperty, setKeyframeProperty] = useState<KeyframeProperty>("opacity");
  const [keyframeValue, setKeyframeValue] = useState("1");
  const [selectedKeyframeTime, setSelectedKeyframeTime] = useState<number | null>(null);
  const [keyframeInterpDraft, setKeyframeInterpDraft] = useState<InterpolationKind>("linear");
  const [keyframeHandleInX] = useState<number>(0.42);
  const [keyframeHandleInY] = useState<number>(0.0);
  const [keyframeHandleOutX] = useState<number>(0.58);
  const [keyframeHandleOutY] = useState<number>(1.0);

  const [selectedSubtitle, setSelectedSubtitle] = useState<SubtitleItem | null>(null);
  const [subtitleTextDraft, setSubtitleTextDraft] = useState<string | null>(null);

  const [trackingMode, setTrackingMode] = useState<TrackingMode>("stabilize");
  const [trackingSmoothness, setTrackingSmoothness] = useState(0.5);
  const [trackingTargetClipId, setTrackingTargetClipId] = useState<number | null>(null);
  const [isAnalyzingTracking, setIsAnalyzingTracking] = useState(false);
  const [showTrackingBox, setShowTrackingBox] = useState(false);

  const saveColor = useCallback(
    async (next: ColorAdjustments) => {
      if (!selectedClip) return;
      setColorDraft(next);
      await perform("clip.set_color", { ...selectedClip, ...next }, "Color settings saved");
    },
    [selectedClip, perform]
  );

  const addEffect = useCallback(
    async (kind: EffectKind, name?: string): Promise<number | null> => {
      if (!selectedClip) return null;
      setBusy(true);
      setNotice("");
      try {
        const res = await runCommand<{ added: boolean; effect_id: number }>("clip.add_effect", { ...selectedClip, kind, name });
        await refreshProject();
        setNotice("Effect added to clip");
        return res?.effect_id ?? null;
      } catch (error) {
        setNotice(String(error));
        return null;
      } finally {
        setBusy(false);
      }
    },
    [selectedClip, setBusy, setNotice, runCommand, refreshProject]
  );

  const removeEffect = useCallback(
    async (effectId: number | string) => {
      if (!selectedClip) return;
      await perform("clip.remove_effect", { ...selectedClip, effect_id: Number(effectId) }, "Effect removed from clip");
    },
    [selectedClip, perform]
  );

  const toggleEffect = useCallback(
    async (effectId: number | string, enabled?: boolean) => {
      if (!selectedClip) return;
      await perform("clip.toggle_effect", { ...selectedClip, effect_id: Number(effectId), enabled }, "Effect toggled");
    },
    [selectedClip, perform]
  );

  const updateEffectParam = useCallback(
    (_id: string, _paramKey: string, _value: number) => {
      // In-memory update
    },
    []
  );

  const updateEffectPayload = useCallback(
    async (effectId: number, payload: EffectPayload) => {
      if (!selectedClip) return;
      await perform("clip.update_effect_payload", { ...selectedClip, effect_id: effectId, payload }, "Effect updated");
    },
    [selectedClip, perform]
  );

  const saveEffectParam = useCallback(
    async (id: string, paramKey: string, value: number) => {
      if (!selectedClip) return;
      await perform("clip.update_effect_param", {
        ...selectedClip,
        effect_id: Number(id),
        param_key: paramKey,
        value,
      });
    },
    [selectedClip, perform]
  );

  const saveKeyframe = useCallback(
    async (
      property: KeyframeProperty,
      value: number,
      timeOverride?: number,
      interpolation?: InterpolationKind,
      handle_in_x?: number | null,
      handle_in_y?: number | null,
      handle_out_x?: number | null,
      handle_out_y?: number | null
    ) => {
      if (!selectedClip || !selectedTimelineClip) return;
      const time = timeOverride !== undefined
        ? timeOverride
        : Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start));
      await perform("clip.set_keyframe", {
        ...selectedClip,
        property,
        time,
        value,
        interpolation,
        handle_in_x,
        handle_in_y,
        handle_out_x,
        handle_out_y,
      }, "Keyframe saved");
      setSelectedKeyframeTime(time);
    },
    [selectedClip, selectedTimelineClip, playhead, perform]
  );

  const deleteKeyframe = useCallback(
    async (property: KeyframeProperty, time: number) => {
      if (!selectedClip) return;
      await perform("clip.remove_keyframe", { ...selectedClip, property, time }, "Keyframe removed");
      if (selectedKeyframeTime === time) {
        setSelectedKeyframeTime(null);
      }
    },
    [selectedClip, perform, selectedKeyframeTime]
  );

  const addSubtitleAtPlayhead = useCallback(async () => {
    const start_ticks = playhead;
    const end_ticks = playhead + 3 * TICKS_PER_SECOND;
    const text = "New Subtitle";
    setBusy(true);
    setNotice("");
    try {
      const res = await runCommand<SubtitleItem>("subtitle.add", { start_ticks, end_ticks, text });
      await refreshProject();
      if (res) {
        setSelectedSubtitle(res);
      }
      setNotice("Subtitle card added at playhead");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [playhead, setBusy, setNotice, runCommand, refreshProject]);

  const removeSubtitle = useCallback(
    async (id: number) => {
      await perform("subtitle.remove", { id }, "Subtitle removed");
      if (selectedSubtitle?.id === id) {
        setSelectedSubtitle(null);
      }
    },
    [perform, selectedSubtitle]
  );

  const updateSubtitle = useCallback(
    async (id: number, updates: { text?: string; start_ticks?: number; end_ticks?: number }) => {
      await perform("subtitle.update", { id, ...updates }, "Subtitle updated");
      setSelectedSubtitle((curr: SubtitleItem | null) => {
        if (curr && curr.id === id) {
          return { ...curr, ...updates };
        }
        return curr;
      });
    },
    [perform]
  );

  const clearSubtitles = useCallback(async () => {
    const confirmed = await confirm("Are you sure you want to delete all subtitles in the project?", {
      title: "Clear all subtitles",
      kind: "warning",
    });
    if (!confirmed) return;
    await perform("subtitle.clear", {}, "All subtitles cleared");
    setSelectedSubtitle(null);
  }, [perform]);

  const importSubtitlesDialog = useCallback(async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Subtitle files (*.srt, *.vtt)", extensions: ["srt", "vtt"] }],
    });
    if (!selected || typeof selected !== "string") return;

    try {
      const items = await invoke<SubtitleItem[]>("import_subtitles_file", { path: selected });
      if (!items || items.length === 0) {
        setNotice("No subtitles found in file");
        return;
      }
      setBusy(true);
      for (const item of items) {
        await runCommand("subtitle.add", {
          start_ticks: item.start_ticks,
          end_ticks: item.end_ticks,
          text: item.text,
        });
      }
      await refreshProject();
      setNotice(`Imported ${items.length} subtitles from ${selected.split("/").pop()}`);
    } catch (err) {
      setNotice(`Failed to import subtitles: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }, [setNotice, setBusy, runCommand, refreshProject]);

  const exportSubtitlesDialog = useCallback(async () => {
    const subtitles = project?.sequence.subtitles ?? [];
    if (subtitles.length === 0) {
      setNotice("No subtitles in sequence to export");
      return;
    }
    const selected = await save({
      defaultPath: `${project?.name ?? "Subtitles"}.srt`,
      filters: [
        { name: "SubRip (.srt)", extensions: ["srt"] },
        { name: "WebVTT (.vtt)", extensions: ["vtt"] },
      ],
    });
    if (!selected || typeof selected !== "string") return;

    try {
      const format = selected.toLowerCase().endsWith(".vtt") ? "vtt" : "srt";
      await invoke("export_subtitles_file", { path: selected, format });
      setNotice(`Exported ${subtitles.length} subtitles to ${selected.split("/").pop()}`);
    } catch (err) {
      setNotice(`Failed to export subtitles: ${String(err)}`);
    }
  }, [project, setNotice]);

  const updateStabilizationSmoothness = useCallback(
    async (val: number) => {
      if (!selectedClip) return;
      setTrackingSmoothness(val);
      await perform("clip.set_stabilization_smoothness", { ...selectedClip, smoothness: val });
    },
    [selectedClip, perform]
  );

  const pinClipToMotionTrack = useCallback(
    async (targetClipId: number | null) => {
      if (!selectedClip) return;
      setTrackingTargetClipId(targetClipId);
      await perform("clip.pin_to_motion_track", { ...selectedClip, target_clip_id: targetClipId });
    },
    [selectedClip, perform]
  );

  const runMotionTracker = useCallback(
    async (forward = true) => {
      if (!selectedClip) return;
      setIsAnalyzingTracking(true);
      try {
        await perform("clip.run_motion_tracker", { ...selectedClip, forward, mode: trackingMode }, "Motion tracking completed");
      } finally {
        setIsAnalyzingTracking(false);
      }
    },
    [selectedClip, perform, trackingMode]
  );

  const saveSpeedRamp = useCallback(
    async (ramp: SpeedRamp) => {
      if (!selectedClip) return;
      await perform("clip.set_speed_ramp", { ...selectedClip, speed_ramp: ramp }, "Speed ramp updated");
    },
    [selectedClip, perform]
  );

  const saveMotionBlur = useCallback(
    async (mb: MotionBlurParams) => {
      if (!selectedClip) return;
      await perform("clip.set_motion_blur", { ...selectedClip, motion_blur: mb }, "Motion blur updated");
    },
    [selectedClip, perform]
  );

  return {
    colorDraft,
    setColorDraft,
    colorWheelsDraft,
    setColorWheelsDraft,
    keyframeProperty,
    setKeyframeProperty,
    keyframeValue,
    setKeyframeValue,
    selectedKeyframeTime,
    setSelectedKeyframeTime,
    keyframeInterpDraft,
    setKeyframeInterpDraft,
    keyframeHandleInX,
    keyframeHandleInY,
    keyframeHandleOutX,
    keyframeHandleOutY,
    selectedSubtitle,
    setSelectedSubtitle,
    subtitleTextDraft,
    setSubtitleTextDraft,
    trackingMode,
    setTrackingMode,
    trackingSmoothness,
    setTrackingSmoothness,
    trackingTargetClipId,
    setTrackingTargetClipId,
    isAnalyzingTracking,
    setIsAnalyzingTracking,
    showTrackingBox,
    setShowTrackingBox,
    saveColor,
    addEffect,
    removeEffect,
    toggleEffect,
    updateEffectPayload,
    updateEffectParam,
    saveEffectParam,
    saveKeyframe,
    deleteKeyframe,
    addSubtitleAtPlayhead,
    removeSubtitle,
    updateSubtitle,
    clearSubtitles,
    importSubtitlesDialog,
    exportSubtitlesDialog,
    updateStabilizationSmoothness,
    pinClipToMotionTrack,
    runMotionTracker,
    saveSpeedRamp,
    saveMotionBlur,
  };
}
