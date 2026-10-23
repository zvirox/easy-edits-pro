import type {
  AudioBus, AudioPluginInstance, AutomationMode, BusTarget, CaptionStyle, CaptionTrack,
  GraphicItem, GraphicTemplate,
  CollaborationState, EffectInstance, HslQualifier, Keyframe, MotionBlurParams,
  MotionTrackingData, MultiCamClip, NodeGraph, RgbCurves, SpeedRamp, SubtitleItem,
  TrackAudioProcessing, TrackAutomation, TrackGroup, SilenceInterval,
} from "../engine/playbackEngine";

export type {
  AudioBus, AudioPluginInstance, AutomationMode, BusTarget, CaptionStyle, CaptionTrack,
  GraphicItem, GraphicTemplate,
  CollaborationState, EffectInstance, HslQualifier, Keyframe, MotionBlurParams,
  MotionTrackingData, MultiCamClip, NodeGraph, RgbCurves, SpeedRamp, SubtitleItem,
  TrackAudioProcessing, TrackAutomation, TrackGroup, SilenceInterval,
};

export type MediaKind = "video" | "audio" | "image" | "title" | "compound_sequence" | "unknown";
export type TitleProperties = {
  text: string;
  subtitle?: string | null;
  style: string;
  font_family: string;
  font_size: number;
  color: string;
  bg_color?: string | null;
};
export type TransitionKind =
  | "crossfade"
  | "dip_to_black"
  | "dip_to_white"
  | "wipe_left"
  | "wipe_right"
  | "slide_left"
  | "slide_right";
export type Transition = {
  id: number;
  kind: TransitionKind;
  cut_at: number;
  duration: number;
};
export type MarkerKind = "comment" | "chapter" | "flash_cue" | "web_link" | "segmentation";
export type Marker = {
  id: number;
  name: string;
  kind: MarkerKind;
  start_ticks: number;
  duration_ticks: number;
  color: string;
  note: string;
};
export type Sequence = {
  id?: number;
  name: string;
  tracks: Track[];
  track_groups?: TrackGroup[];
  mark_in: number | null;
  mark_out: number | null;
  markers?: Marker[];
  caption_tracks?: CaptionTrack[];
  graphics?: GraphicItem[];
  multicam_clips?: MultiCamClip[];
  buses?: AudioBus[];
  node_graph?: NodeGraph | null;
};
export type MediaBin = {
  id: number;
  name: string;
  parent_id?: number | null;
  color?: string | null;
};
export type MediaAsset = {
  id: number;
  name: string;
  path: string;
  kind: MediaKind;
  duration: number | null;
  width: number | null;
  height: number | null;
  fps?: number | null;
  audio_channels?: number | null;
  codec?: string | null;
  bin_id?: number | null;
  color_label?: string | null;
  color_tag?: string | null;
  tags?: string[] | null;
  comment?: string | null;
  scene?: string | null;
  take?: string | null;
  is_missing?: boolean;
  title?: TitleProperties | null;
  waveform?: number[] | null;
  nested_sequence?: Sequence | null;
};
export type BackendColorWheels = {
  lift: { red: number; green: number; blue: number; luma: number };
  gamma: { red: number; green: number; blue: number; luma: number };
  gain: { red: number; green: number; blue: number; luma: number };
  offset: { red: number; green: number; blue: number; luma: number };
};
export type ColorAdjustments = {
  exposure: number;
  contrast: number;
  saturation: number;
  hue?: number;
  wheels?: BackendColorWheels;
  curves?: RgbCurves;
  qualifier?: HslQualifier;
};
export type Clip = {
  media_id: number;
  timeline_start: number;
  source_in: number;
  duration: number;
  speed?: number;
  group_id?: number | null;
  color: ColorAdjustments;
  effects?: EffectInstance[];
  keyframes: Keyframe[];
  multicam_id?: number | null;
  multicam_angle_id?: number | null;
  tracking?: MotionTrackingData | null;
  speed_ramp?: SpeedRamp | null;
  motion_blur?: MotionBlurParams | null;
  node_graph?: NodeGraph | null;
  compound_sequence_id?: number | null;
};
export type Track = {
  id: number;
  group_id?: number | null;
  kind: "video" | "audio";
  name: string;
  gain_db: number;
  pan?: number;
  muted?: boolean;
  solo?: boolean;
  locked?: boolean;
  audio_processing?: TrackAudioProcessing;
  bus?: BusTarget;
  audio_bus?: string;
  linked_group_id?: number | null;
  automation?: TrackAutomation;
  plugins?: AudioPluginInstance[];
  clips: Clip[];
  transitions?: Transition[];
};
export interface HardwareEncoderInfo {
  hasHardwareEncoder: boolean;
  preferredEncoder: string;
  encoderType: string;
  description: string;
  availableEncoders: string[];
}
export type LoudnessStandard = "ebu_r128" | "streaming" | "atsc_a85";
export type LoudnessReport = {
  momentaryLufs: number;
  shortTermLufs: number;
  integratedLufs: number;
  loudnessRangeLu: number;
  truePeakDbtp: number;
  targetLufs: number;
  isCompliant: boolean;
  gainRecommendationDb: number;
};
export type ProjectDocument = {
  project: {
    name: string;
    bins?: MediaBin[];
    media: MediaAsset[];
    sequence: Sequence;
    sequences?: Sequence[];
    graphic_templates?: GraphicTemplate[];
    collaboration?: CollaborationState;
    settings: { width: number; height: number; frame_rate: { numerator: number; denominator: number }; audio_sample_rate: number };
  };
};
export type ProjectStatus = { document: ProjectDocument; path: string | null; is_dirty: boolean };

export type RecoveryStatus = { available: boolean; projectName?: string; savedAt?: number };

// F1: one canonical SelectedClip (see types.ts); re-exported for the
// app-level modules that import their UI types from here.
export type { SelectedClip } from "../types";

export type DavinciWorkspace =
  | "media"
  | "edit"
  | "fusion"
  | "color"
  | "fairlight"
  | "audio"
  | "effects"
  | "curves"
  | "keyframes"
  | "deliver";

export interface GpuMetrics {
  fpsMeasured: number;
  surfaceWidth: number;
  surfaceHeight: number;
}

export type FilmoraTab = "media" | "titles" | "transitions" | "audio" | "stock" | "ai";

export type OpenverseAsset = {
  id: string;
  title: string;
  mediaUrl: string;
  sourceUrl: string;
  thumbnailUrl: string | null;
  creator: string | null;
  license: string | null;
  licenseUrl: string | null;
};

export type TimelineTool =
  | "select"
  | "trackForward"
  | "trackBackward"
  | "ripple"
  | "rolling"
  | "razor"
  | "slip"
  | "slide"
  | "rateStretch"
  | "remix"
  | "pen"
  | "hand"
  | "zoom"
  | "rectangle"
  | "ellipse"
  | "type"
  | "verticalType";

export const TRACK_LABEL_WIDTH = 114;
export const TICKS_PER_SECOND = 254_016_000_000;

export function getTrackGaps(clips: Clip[]): Array<{ start: number; end: number }> {
  if (clips.length === 0) return [];
  const sorted = [...clips].sort((a, b) => a.timeline_start - b.timeline_start);
  const gaps: Array<{ start: number; end: number }> = [];
  let currentEnd = 0;
  for (const clip of sorted) {
    if (clip.timeline_start > currentEnd) {
      gaps.push({ start: currentEnd, end: clip.timeline_start });
    }
    currentEnd = Math.max(currentEnd, clip.timeline_start + clip.duration);
  }
  return gaps;
}
