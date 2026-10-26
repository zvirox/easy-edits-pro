import type {
  TrackAudioProcessing,
  LutParams,
  InterpolationKind,
  SubtitleItem,
  Keyframe,
  KeyframeProperty,
  MultiCamClip,
  TrackingMode,
  TrackPoint,
  MotionTrackingData,
  BusTarget,
  AutomationMode,
  AutomationInterpolation,
  AutomationPoint,
  TrackAutomation,
  AudioBus,
  CompositingNode,
  CompositingNodeKind,
  NodeConnection,
  RotoSplineMask,
  RotoSplinePoint,
  NodeGraph,
  SpeedRamp,
  MotionBlurParams,
  SpeechSegment,
  SilenceInterval,
  RippleDeleteReport,
  PresentationMetrics,
  InteropFormat,
  TrackGroup,
  RgbCurves,
  HslQualifier,
  CustomShaderParams,
  EffectInstance,
  EffectKind,
  CaptionTrack,
  CaptionStyle,
  GraphicItem,
  GraphicTemplate,
} from "./engine/playbackEngine";

export type {
  TrackAudioProcessing,
  LutParams,
  InterpolationKind,
  SubtitleItem,
  CaptionTrack,
  CaptionStyle,
  GraphicItem,
  GraphicTemplate,
  Keyframe,
  KeyframeProperty,
  MultiCamClip,
  TrackingMode,
  TrackPoint,
  MotionTrackingData,
  BusTarget,
  AutomationMode,
  AutomationInterpolation,
  AutomationPoint,
  TrackAutomation,
  AudioBus,
  CompositingNode,
  CompositingNodeKind,
  NodeConnection,
  RotoSplineMask,
  RotoSplinePoint,
  NodeGraph,
  SpeedRamp,
  MotionBlurParams,
  SpeechSegment,
  SilenceInterval,
  RippleDeleteReport,
  PresentationMetrics,
  InteropFormat,
  TrackGroup,
  RgbCurves,
  HslQualifier,
  CustomShaderParams,
  EffectInstance,
  EffectKind,
};

export type JobState =
  | { status: "queued" }
  | { status: "running"; progress: number; phase: string }
  | { status: "completed"; output?: string | null }
  | { status: "failed"; error: string }
  | { status: "cancelled" };

export interface JobInfo {
  id: number;
  label: string;
  kind: { type: string; [key: string]: unknown };
  priority: "background" | "normal" | "high" | "immediate";
  state: JobState;
  created_at: number;
  started_at?: number | null;
  completed_at?: number | null;
}

export type Workspace = "edit" | "color" | "effects" | "audio" | "keyframes" | "export" | "library" | "ai";
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
export type DavinciWorkspace = "media" | "edit" | "fusion" | "color" | "audio" | "effects" | "keyframes" | "deliver";
export type FilmoraTab = "media" | "titles" | "transitions" | "audio" | "stock" | "ai";
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
  proxy_path?: string | null;
  colorspace?: string | null;
  gamma?: string | null;
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
  label?: string;
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

export type Sequence = {
  id?: number;
  name: string;
  tracks: Track[];
  track_groups?: TrackGroup[];
  mark_in: number | null;
  mark_out: number | null;
  caption_tracks?: CaptionTrack[];
  graphics?: GraphicItem[];
  multicam_clips?: MultiCamClip[];
  buses?: AudioBus[];
  node_graph?: NodeGraph | null;
};

export type ProjectDocument = {
  project: {
    name: string;
    notes?: string;
    bins?: MediaBin[];
    media: MediaAsset[];
    sequence: Sequence;
    sequences?: Sequence[];
    graphic_templates?: GraphicTemplate[];
    settings: { width: number; height: number; frame_rate: { numerator: number; denominator: number }; audio_sample_rate: number; use_proxies?: boolean };
  };
};

export type ProjectStatus = { document: ProjectDocument; path: string | null; is_dirty: boolean };
export type CommandStatus = { enabled: boolean; reason: string | null };
export type RecoveryStatus = { available: boolean; projectName?: string; savedAt?: number };
export type MediaAvailability = { mediaId: number; missing: boolean };
export type CommandSpec = {
  id: string;
  label: string;
  menu: string[];
  shortcut: string | null;
  params: string;
  category: string;
  mutating: boolean;
  undoGroup: "none" | "transaction" | "gesture";
  aliases: string[];
  since: string;
  shipped: boolean;
  uiOnly: boolean;
};
export type SelectedClip = { track_id: number; clip_index: number };
export type ClipPointerDrag = { pointerId: number; trackId: number; clipIndex: number; startX: number; startY: number; startTime: number; duration: number };
export type MediaPointerDrag = { pointerId: number; mediaId: number; startX: number; startY: number; dragging: boolean };
export type TimelineScrub = { pointerId: number; axis: HTMLElement };
export type OpenverseAsset = { id: string; title: string; mediaUrl: string; sourceUrl: string; thumbnailUrl: string | null; creator: string | null; license: string | null; licenseUrl: string | null };
export type RecentProject = { name: string; path: string };
export type AppStage = "welcome" | "loading" | "editor";

export const recentProjectsKey = "easy-edits-pro.recent-projects.v1";

export const resolutionPresets = [
  { value: "1920x1080", label: "Full HD · 1920 × 1080", width: 1920, height: 1080 },
  { value: "3840x2160", label: "Ultra HD · 3840 × 2160", width: 3840, height: 2160 },
  { value: "2560x1440", label: "QHD · 2560 × 1440", width: 2560, height: 1440 },
  { value: "1280x720", label: "HD · 1280 × 720", width: 1280, height: 720 },
  { value: "1080x1920", label: "Vertical · 1080 × 1920", width: 1080, height: 1920 },
  { value: "1080x1080", label: "Square · 1080 × 1080 (1:1)", width: 1080, height: 1080 },
  { value: "custom", label: "Custom resolution", width: 1920, height: 1080 },
];

export const exportPresets = [
  {
    id: "yt_4k",
    name: "YouTube 4K UHD",
    subtitle: "3840 × 2160 · 60 fps",
    resolution: "3840x2160",
    frameRate: "60/1",
    format: "mp4" as const,
    icon: "▶",
  },
  {
    id: "yt_1080p",
    name: "YouTube 1080p",
    subtitle: "1920 × 1080 · 60 fps",
    resolution: "1920x1080",
    frameRate: "60/1",
    format: "mp4" as const,
    icon: "▶",
  },
  {
    id: "tiktok_reels",
    name: "TikTok / Reels",
    subtitle: "1080 × 1920 · 9:16 Vertical",
    resolution: "1080x1920",
    frameRate: "30/1",
    format: "mp4" as const,
    icon: "📱",
  },
  {
    id: "instagram_square",
    name: "Instagram Square",
    subtitle: "1080 × 1080 · 1:1 Feed",
    resolution: "1080x1080",
    frameRate: "30/1",
    format: "mp4" as const,
    icon: "📷",
  },
  {
    id: "cinema_4k",
    name: "Cinema 4K DCI",
    subtitle: "3840 × 2160 · 24 fps Film",
    resolution: "3840x2160",
    frameRate: "24/1",
    format: "mp4" as const,
    icon: "🎬",
  },
  {
    id: "web_hd",
    name: "Web HD 720p",
    subtitle: "1280 × 720 · 30 fps Fast",
    resolution: "1280x720",
    frameRate: "30/1",
    format: "mp4" as const,
    icon: "🌐",
  },
];

export const frameRateOptions = [
  { value: "24000/1001", label: "23.976 fps", numerator: 24_000, denominator: 1_001 },
  { value: "24/1", label: "24 fps", numerator: 24, denominator: 1 },
  { value: "25/1", label: "25 fps", numerator: 25, denominator: 1 },
  { value: "30000/1001", label: "29.97 fps", numerator: 30_000, denominator: 1_001 },
  { value: "30/1", label: "30 fps", numerator: 30, denominator: 1 },
  { value: "50/1", label: "50 fps", numerator: 50, denominator: 1 },
  { value: "60000/1001", label: "59.94 fps", numerator: 60_000, denominator: 1_001 },
  { value: "60/1", label: "60 fps", numerator: 60, denominator: 1 },
];

export const exportFrameRateOptions = [
  { value: "source", label: "Match sequence" },
  ...frameRateOptions.map((item) => ({ value: `${item.numerator}/${item.denominator}`, label: item.label })),
];

export const TICKS_PER_SECOND = 254_016_000_000;

export const TRACK_LABEL_WIDTH = 114;

export const COLOR_PRESETS: { id: string; label: string; icon: string; exposure: number; contrast: number; saturation: number }[] = [
  { id: "neutral", label: "Neutral Rec.709", icon: "⚪", exposure: 0, contrast: 1.0, saturation: 1.0 },
  { id: "teal_orange", label: "Teal & Orange", icon: "🎬", exposure: 0.2, contrast: 1.25, saturation: 1.35 },
  { id: "film_noir", label: "Black & White Noir", icon: "🖤", exposure: -0.1, contrast: 1.45, saturation: 0.0 },
  { id: "vintage_warm", label: "Warm Vintage", icon: "🌅", exposure: 0.15, contrast: 1.1, saturation: 1.2 },
  { id: "bleach_bypass", label: "Bleach Bypass", icon: "📽️", exposure: 0.1, contrast: 1.5, saturation: 0.5 },
  { id: "cyberpunk", label: "Cyberpunk Glow", icon: "🔮", exposure: 0.25, contrast: 1.3, saturation: 1.6 },
];

export const DEFAULT_COLOR_WHEELS: BackendColorWheels = {
  lift: { red: 0, green: 0, blue: 0, luma: 0 },
  gamma: { red: 0, green: 0, blue: 0, luma: 0 },
  gain: { red: 0, green: 0, blue: 0, luma: 0 },
  offset: { red: 0, green: 0, blue: 0, luma: 0 },
};

export interface BatchQueueItem {
  id: number;
  name: string;
  format: string;
  resolution: string;
  status: "queued" | "running" | "paused" | "completed" | "failed" | "cancelled";
  progress: number;
  outputPath: string;
}

