import type { CSSProperties } from "react";

export type TransitionKind =
  | "crossfade"
  | "dip_to_black"
  | "dip_to_white"
  | "wipe_left"
  | "wipe_right"
  | "slide_left"
  | "slide_right";

export interface Transition {
  id: number;
  kind: TransitionKind;
  cut_at: number;
  duration: number;
}

export type KeyframeProperty = "position_x" | "position_y" | "scale" | "rotation" | "opacity";

export type InterpolationKind = "linear" | "ease_in" | "ease_out" | "ease_in_out" | "bezier";

export interface Keyframe {
  property: KeyframeProperty;
  time: number;
  value: number;
  interpolation?: InterpolationKind;
  handle_in_x?: number | null;
  handle_in_y?: number | null;
  handle_out_x?: number | null;
  handle_out_y?: number | null;
}

export interface ColorWheelParam {
  red: number;
  green: number;
  blue: number;
  luma: number;
}

export interface ColorWheels {
  lift: ColorWheelParam;
  gamma: ColorWheelParam;
  gain: ColorWheelParam;
  offset: ColorWheelParam;
}

export interface CurvePoint {
  x: number;
  y: number;
}

export interface RgbCurves {
  master: CurvePoint[];
  red: CurvePoint[];
  green: CurvePoint[];
  blue: CurvePoint[];
}

export interface HslQualifier {
  enabled: boolean;
  hue_center: number;
  hue_width: number;
  hue_softness: number;
  sat_center: number;
  sat_width: number;
  sat_softness: number;
  luma_center: number;
  luma_width: number;
  luma_softness: number;
  invert: boolean;
  blur_radius: number;
  hue_shift: number;
  sat_boost: number;
  exposure_shift: number;
}

export interface TrackGroup {
  id: number;
  name: string;
  color: string;
  collapsed: boolean;
  muted: boolean;
  solo: boolean;
  locked: boolean;
}

export interface ColorAdjustments {
  exposure: number;
  contrast: number;
  saturation: number;
  hue?: number;
  wheels?: ColorWheels;
  curves?: RgbCurves;
  qualifier?: HslQualifier;
}

export type MultiCamSyncMode = "timecode" | "audio_waveform" | "marker";

export interface MultiCamAngle {
  id: number;
  name: string;
  media_id: number;
  sync_offset_ticks: number;
}

export interface MultiCamClip {
  id: number;
  name: string;
  angles: MultiCamAngle[];
  active_angle_id: number;
  sync_mode?: MultiCamSyncMode;
}

export type TrackingMode = "transform" | "stabilize" | "pin";

export interface TrackPoint {
  time: number;
  x: number;
  y: number;
  confidence: number;
}

export interface MotionTrackingData {
  id: number;
  name: string;
  mode: TrackingMode;
  points: TrackPoint[];
  pinned_clip_id?: number | null;
  smoothness: number;
}

export type BusTarget = "master" | "submix1" | "submix2" | "aux_fx";

export type AutomationMode = "off" | "read" | "touch" | "latch" | "write";

export interface AutomationPoint {
  time: number;
  value: number;
}

export interface TrackAutomation {
  mode: AutomationMode;
  volume_points: AutomationPoint[];
  pan_points: AutomationPoint[];
}

export interface AudioPluginInstance {
  id: number;
  name: string;
  plugin_format: string;
  enabled: boolean;
  preset: string;
  params: Record<string, number>;
}

export interface AudioBus {
  target: BusTarget;
  name: string;
  gain_db: number;
  muted: boolean;
  solo: boolean;
  processing: TrackAudioProcessing;
}

export type EffectKind =
  | "color_correction"
  | "gaussian_blur"
  | "crop"
  | "vignette"
  | "chroma_key"
  | "lut"
  | "custom_shader";

export interface ColorCorrectionParams {
  exposure: number;
  contrast: number;
  saturation: number;
  hue: number;
  wheels?: ColorWheels;
}

export interface GaussianBlurParams {
  sigma: number;
}

export interface CropParams {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export interface VignetteParams {
  intensity: number;
  radius: number;
  softness: number;
}

export interface ChromaKeyParams {
  color: string;
  tolerance: number;
  softness: number;
  spill_suppression: number;
}

export interface LutParams {
  name: string;
  path?: string | null;
  intensity: number;
  preset_id?: string | null;
}

export interface CustomShaderParams {
  name: string;
  shader_code: string;
  preset_id?: string | null;
  uniforms?: Record<string, number>;
}

export type EffectPayload =
  | { kind: "color_correction"; params: ColorCorrectionParams }
  | { kind: "gaussian_blur"; params: GaussianBlurParams }
  | { kind: "crop"; params: CropParams }
  | { kind: "vignette"; params: VignetteParams }
  | { kind: "chroma_key"; params: ChromaKeyParams }
  | { kind: "lut"; params: LutParams }
  | { kind: "custom_shader"; params: CustomShaderParams };

export interface EffectInstance {
  id: number;
  name: string;
  enabled: boolean;
  kind: EffectKind;
  params: ColorCorrectionParams | GaussianBlurParams | CropParams | VignetteParams | ChromaKeyParams | LutParams | CustomShaderParams;
}

export type CompositingNodeKind =
  | "merge"
  | "transform"
  | "keyer"
  | "mask"
  | "particles"
  | "media_source"
  | "output_sink";

export interface CompositingNode {
  id: number;
  kind: CompositingNodeKind;
  title: string;
  pos_x: number;
  pos_y: number;
  params?: Record<string, unknown>;
  enabled?: boolean;
}

export interface NodeConnection {
  from_node: number;
  from_socket: string;
  to_node: number;
  to_socket: string;
}

export interface RotoSplinePoint {
  x: number;
  y: number;
  handle_in?: [number, number];
  handle_out?: [number, number];
}

export interface RotoSplineMask {
  id: number;
  name: string;
  closed?: boolean;
  inverted?: boolean;
  feather?: number;
  opacity?: number;
  points: RotoSplinePoint[];
}

export interface NodeGraph {
  nodes: CompositingNode[];
  connections: NodeConnection[];
  masks?: RotoSplineMask[];
}

export interface SpeedRampPoint {
  time: number;
  speed_factor: number;
  bezier_tangent?: [number, number];
}

export interface SpeedRamp {
  points: SpeedRampPoint[];
  optical_flow_interpolation?: boolean;
  frame_blend?: boolean;
}

export interface MotionBlurParams {
  enabled?: boolean;
  shutter_angle_deg?: number;
  samples?: number;
}

export type LockTargetKind = "bin" | "sequence" | "track" | "clip";

export interface UserLock {
  user_id: string;
  user_name: string;
  user_color?: string;
  target_kind: LockTargetKind;
  target_id: number;
  locked_at: number;
}

export interface TimelineChangeEntry {
  id: number;
  user_id: string;
  user_name: string;
  timestamp: number;
  action: string;
  summary: string;
}

export interface CollaborationState {
  session_id: string;
  active_users: string[];
  locks: UserLock[];
  change_log: TimelineChangeEntry[];
}

export interface Clip {
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
  node_graph?: NodeGraph | null;
  speed_ramp?: SpeedRamp | null;
  motion_blur?: MotionBlurParams | null;
  compound_sequence_id?: number | null;
}

export interface TrackAudioProcessing {
  eq_enabled?: boolean;
  eq_low_gain?: number;
  eq_mid_gain?: number;
  eq_high_gain?: number;
  comp_enabled?: boolean;
  comp_threshold?: number;
  comp_ratio?: number;
  comp_attack?: number;
  comp_release?: number;
  denoise_enabled?: boolean;
  denoise_amount?: number;
}

export interface Track {
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
}

export interface TitleProperties {
  text: string;
  subtitle?: string | null;
  style: string;
  font_family: string;
  font_size: number;
  color: string;
  bg_color?: string | null;
}

export interface Sequence {
  id?: number;
  name: string;
  track_groups?: TrackGroup[];
  tracks: Track[];
  mark_in: number | null;
  mark_out: number | null;
  subtitles?: SubtitleItem[];
  multicam_clips?: MultiCamClip[];
  buses?: AudioBus[];
  node_graph?: NodeGraph | null;
}

export interface MediaBin {
  id: number;
  name: string;
  parent_id?: number | null;
  color?: string | null;
}

export interface MediaAsset {
  id: number;
  name: string;
  path: string;
  kind: "video" | "audio" | "image" | "title" | "compound_sequence" | "unknown";
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
}

export interface SubtitleItem {
  id: number;
  start_ticks: number;
  end_ticks: number;
  text: string;
}

export interface ProjectDocument {
  project: {
    name: string;
    media: MediaAsset[];
    bins?: MediaBin[];
    sequence: Sequence;
    sequences?: Sequence[];
    settings: {
      width: number;
      height: number;
      frame_rate: { numerator: number; denominator: number };
      audio_sample_rate: number;
    };
    collaboration?: CollaborationState;
  };
}

export type LoudnessStandard = "ebu_r128" | "streaming" | "atsc_a85";

export interface LoudnessReport {
  momentaryLufs: number;
  shortTermLufs: number;
  integratedLufs: number;
  loudnessRangeLu: number;
  truePeakDbtp: number;
  targetLufs: number;
  isCompliant: boolean;
  gainRecommendationDb: number;
}

export interface TransitionBlend {
  active: boolean;
  kind: TransitionKind;
  progress: number; // 0.0 to 1.0
  deckAStyle: CSSProperties;
  deckBStyle: CSSProperties;
}

export interface DeckConfig {
  mediaId: number | null;
  src: string;
  currentTime: number;
  visible: boolean;
  style: CSSProperties;
  vignette?: { intensity: number; radius: number; softness: number } | null;
  chromaKey?: ChromaKeyParams | null;
  clip: Clip | null;
  track: Track | null;
  isPreload: boolean;
}

export interface ActiveAudioClip {
  track: Track;
  clip: Clip;
  media: MediaAsset;
  src: string;
  currentTime: number;
  gainDb: number;
}

export interface TimelinePlaybackSnapshot {
  playhead: number;
  sequenceDuration: number;
  deckA: DeckConfig;
  deckB: DeckConfig;
  activeDeck: "A" | "B";
  inTransition: boolean;
  transitionKind: TransitionKind | null;
  activeAudioClips: ActiveAudioClip[];
  activeTitleClip: { track: Track; clip: Clip; media: MediaAsset; title: TitleProperties } | null;
  activeSubtitle: SubtitleItem | null;
  hasVideoMedia: boolean;
}

export interface SpeechWord {
  word: string;
  startTicks: number;
  endTicks: number;
  confidence: number;
  isFiller: boolean;
}

export interface SpeechSegment {
  id: number;
  startTicks: number;
  endTicks: number;
  text: string;
  speaker?: string | null;
  confidence: number;
  words: SpeechWord[];
}

export interface SilenceInterval {
  startTicks: number;
  endTicks: number;
  durationMs: number;
}

export interface RippleDeleteReport {
  deletedRangesCount: number;
  totalDurationRemovedTicks: number;
  tracksAffected: number;
  clipsModified: number;
  subtitlesAdjusted: number;
}

export interface PresentationMetrics {
  frameIndex: number;
  presentationTimeUs: number;
  fpsMeasured: number;
  bandwidthGbps: number;
  surfaceWidth: number;
  surfaceHeight: number;
  droppedFrames: number;
  isHardwareAccelerated: boolean;
}

export type InteropFormat = "edl" | "fcpxml" | "premiere_xml" | "otio";
