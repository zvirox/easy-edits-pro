import type {
  AudioBus, AudioPluginInstance, BusTarget, CollaborationState, EffectInstance,
  HslQualifier, Keyframe, MotionBlurParams, MotionTrackingData, MultiCamClip, NodeGraph, RgbCurves,
  SpeedRamp, SubtitleItem, TrackAudioProcessing, TrackAutomation, TrackGroup,
} from "../engine/playbackEngine";

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
export type Sequence = {
  id?: number;
  name: string;
  tracks: Track[];
  track_groups?: TrackGroup[];
  mark_in: number | null;
  mark_out: number | null;
  subtitles?: SubtitleItem[];
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
  tags?: string[] | null;
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
    collaboration?: CollaborationState;
    settings: { width: number; height: number; frame_rate: { numerator: number; denominator: number }; audio_sample_rate: number };
  };
};
export type ProjectStatus = { document: ProjectDocument; path: string | null; is_dirty: boolean };

export type RecoveryStatus = { available: boolean; projectName?: string; savedAt?: number };
