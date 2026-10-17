import { type CSSProperties, type MouseEvent, type PointerEvent, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { LoadingButton } from "./components/interior/loading-button";
import { VideoScopes } from "./components/VideoScopes";
import { ColorWheels, type ColorWheelsData, DEFAULT_COLOR_WHEELS } from "./components/ColorWheels";
import {
  audioClockMaster,
  evaluateTimelinePlayback,
  type TimelinePlaybackSnapshot,
  type EffectKind,
  type EffectPayload,
  type EffectInstance,
  type ColorCorrectionParams,
  type GaussianBlurParams,
  type CropParams,
  type VignetteParams,
  type ChromaKeyParams,
  type TrackAudioProcessing,
} from "./engine/playbackEngine";

type JobState =
  | { status: "queued" }
  | { status: "running"; progress: number; phase: string }
  | { status: "completed"; output?: string | null }
  | { status: "failed"; error: string }
  | { status: "cancelled" };

interface JobInfo {
  id: number;
  label: string;
  kind: { type: string; [key: string]: unknown };
  priority: "background" | "normal" | "high" | "immediate";
  state: JobState;
  created_at: number;
  started_at?: number | null;
  completed_at?: number | null;
}

type Workspace = "edit" | "color" | "effects" | "audio" | "keyframes" | "export" | "library" | "ai";
type TimelineTool = "select" | "trackForward" | "ripple" | "rolling" | "razor" | "slip" | "slide" | "rateStretch" | "pen" | "hand" | "zoom";
type DavinciWorkspace = "media" | "edit" | "color" | "audio" | "effects" | "deliver";
type FilmoraTab = "media" | "titles" | "transitions" | "audio" | "stock" | "ai";
type MediaKind = "video" | "audio" | "image" | "title" | "unknown";
type TitleProperties = {
  text: string;
  subtitle?: string | null;
  style: string;
  font_family: string;
  font_size: number;
  color: string;
  bg_color?: string | null;
};
type TransitionKind =
  | "crossfade"
  | "dip_to_black"
  | "dip_to_white"
  | "wipe_left"
  | "wipe_right"
  | "slide_left"
  | "slide_right";
type Transition = {
  id: number;
  kind: TransitionKind;
  cut_at: number;
  duration: number;
};
type MediaAsset = {
  id: number;
  name: string;
  path: string;
  kind: MediaKind;
  duration: number | null;
  width: number | null;
  height: number | null;
  title?: TitleProperties | null;
  waveform?: number[] | null;
};
type ColorAdjustments = { exposure: number; contrast: number; saturation: number; hue?: number };
type KeyframeProperty = "position_x" | "position_y" | "scale" | "rotation" | "opacity";
type Keyframe = { property: KeyframeProperty; time: number; value: number };
type Clip = {
  media_id: number;
  timeline_start: number;
  source_in: number;
  duration: number;
  speed?: number;
  group_id?: number | null;
  color: ColorAdjustments;
  effects?: EffectInstance[];
  keyframes: Keyframe[];
};
type Track = {
  id: number;
  kind: "video" | "audio";
  name: string;
  gain_db: number;
  pan?: number;
  muted?: boolean;
  solo?: boolean;
  locked?: boolean;
  audio_processing?: TrackAudioProcessing;
  clips: Clip[];
  transitions?: Transition[];
};
interface HardwareEncoderInfo {
  hasHardwareEncoder: boolean;
  preferredEncoder: string;
  encoderType: string;
  description: string;
  availableEncoders: string[];
}
type ProjectDocument = {
  project: {
    name: string;
    media: MediaAsset[];
    sequence: { name: string; tracks: Track[]; mark_in: number | null; mark_out: number | null };
    settings: { width: number; height: number; frame_rate: { numerator: number; denominator: number }; audio_sample_rate: number };
  };
};
type ProjectStatus = { document: ProjectDocument; path: string | null; is_dirty: boolean };
type CommandStatus = { enabled: boolean; reason: string | null };
type RecoveryStatus = { available: boolean; projectName?: string; savedAt?: number };
type MediaAvailability = { mediaId: number; missing: boolean };
type CommandSpec = { id: string; label: string; menu: string[]; shortcut: string | null; mutating: boolean };
type SelectedClip = { track_id: number; clip_index: number };
type ClipPointerDrag = { pointerId: number; trackId: number; clipIndex: number; startX: number; startY: number; startTime: number; duration: number };
type MediaPointerDrag = { pointerId: number; mediaId: number; startX: number; startY: number; dragging: boolean };
type TimelineScrub = { pointerId: number; axis: HTMLElement };
type OpenverseAsset = { id: string; title: string; mediaUrl: string; sourceUrl: string; thumbnailUrl: string | null; creator: string | null; license: string | null; licenseUrl: string | null };
type RecentProject = { name: string; path: string };
type AppStage = "welcome" | "loading" | "editor";

const recentProjectsKey = "easy-edits-pro.recent-projects.v1";
const resolutionPresets = [
  { value: "1920x1080", label: "Full HD · 1920 × 1080", width: 1920, height: 1080 },
  { value: "3840x2160", label: "Ultra HD · 3840 × 2160", width: 3840, height: 2160 },
  { value: "2560x1440", label: "QHD · 2560 × 1440", width: 2560, height: 1440 },
  { value: "1280x720", label: "HD · 1280 × 720", width: 1280, height: 720 },
  { value: "1080x1920", label: "Vertical · 1080 × 1920", width: 1080, height: 1920 },
  { value: "1080x1080", label: "Square · 1080 × 1080 (1:1)", width: 1080, height: 1080 },
  { value: "custom", label: "Custom resolution", width: 1920, height: 1080 },
];

const exportPresets = [
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
const frameRateOptions = [
  { value: "24000/1001", label: "23.976 fps", numerator: 24_000, denominator: 1_001 },
  { value: "24/1", label: "24 fps", numerator: 24, denominator: 1 },
  { value: "25/1", label: "25 fps", numerator: 25, denominator: 1 },
  { value: "30000/1001", label: "29.97 fps", numerator: 30_000, denominator: 1_001 },
  { value: "30/1", label: "30 fps", numerator: 30, denominator: 1 },
  { value: "50/1", label: "50 fps", numerator: 50, denominator: 1 },
  { value: "60000/1001", label: "59.94 fps", numerator: 60_000, denominator: 1_001 },
  { value: "60/1", label: "60 fps", numerator: 60, denominator: 1 },
];
const exportFrameRateOptions = [
  { value: "source", label: "Match sequence" },
  ...frameRateOptions.map((item) => ({ value: `${item.numerator}/${item.denominator}`, label: item.label })),
];

const TICKS_PER_SECOND = 254_016_000_000;
const workspaces: { id: Workspace; label: string }[] = [
  { id: "edit", label: "Edit" },
  { id: "color", label: "Color" },
  { id: "effects", label: "Effects" },
  { id: "audio", label: "Audio" },
  { id: "keyframes", label: "Keyframes" },
  { id: "export", label: "Export" },
  { id: "library", label: "Asset library" },
  { id: "ai", label: "AI assistant" },
];

const davinciWorkspaces: { id: DavinciWorkspace; label: string; shortcut: string; icon: IconName }[] = [
  { id: "media", label: "Media", shortcut: "Shift+1", icon: "media" },
  { id: "edit", label: "Edit", shortcut: "Shift+2", icon: "edit" },
  { id: "color", label: "Color", shortcut: "Shift+3", icon: "color" },
  { id: "audio", label: "Audio", shortcut: "Shift+4", icon: "audio" },
  { id: "effects", label: "Effects", shortcut: "Shift+5", icon: "effects" },
  { id: "deliver", label: "Deliver", shortcut: "Shift+6", icon: "deliver" },
];

const filmoraTabs: { id: FilmoraTab; label: string; icon: IconName; tooltip: string }[] = [
  { id: "media", label: "Media", icon: "media", tooltip: "Project Media Bin" },
  { id: "titles", label: "Titles", icon: "title", tooltip: "Titles & Lower-Thirds" },
  { id: "transitions", label: "Transitions", icon: "transition", tooltip: "Visual Transitions" },
  { id: "audio", label: "Audio", icon: "audio", tooltip: "Audio & SFX Library" },
  { id: "stock", label: "Stock", icon: "media", tooltip: "Openverse Stock Media" },
  { id: "ai", label: "AI Tools", icon: "effects", tooltip: "Local AI Assistant" },
];

const titlePresets = [
  {
    id: "lower_third",
    name: "Broadcast Lower-Third",
    desc: "Presenter name & role banner",
    preview: "John Doe · Lead Editor",
    style: "lower_third",
    text: "Presenter Name",
    subtitle: "Lead Video Editor",
  },
  {
    id: "minimal_pill",
    name: "Minimal Pill",
    desc: "Modern frosted floating badge",
    preview: "✦ MINIMAL PILL",
    style: "minimal_pill",
    text: "Minimal Pill Headline",
    subtitle: "Subtle contextual note",
  },
  {
    id: "center_bold",
    name: "Cinematic Headline",
    desc: "Bold high-impact centered title",
    preview: "EPISODE 01",
    style: "center_bold",
    text: "CINEMATIC HEADLINE",
    subtitle: "A NEW CHAPTER BEGINS",
  },
  {
    id: "cyber_neon",
    name: "Cyber Neon",
    desc: "Vibrant glowing title effect",
    preview: "⚡ CYBER NEON",
    style: "cyber_neon",
    text: "CYBER NEON 2026",
    subtitle: "NIGHT CITY PROTOCOL",
  },
  {
    id: "headline_subtitle",
    name: "Documentary Card",
    desc: "Headline with location subtext",
    preview: "DOCUMENTARY",
    style: "headline_subtitle",
    text: "Documentary Chapter",
    subtitle: "Location and historical context",
  },
  {
    id: "boxed_banner",
    name: "Framed Banner",
    desc: "Aesthetic boxed caption overlay",
    preview: "[ FRAMED BOX ]",
    style: "boxed_banner",
    text: "Framed Aesthetic Banner",
    subtitle: "Curated sequence notes",
  },
];

const transitionPresets: { id: TransitionKind; name: string; desc: string; iconSymbol: string }[] = [
  { id: "crossfade", name: "Crossfade", desc: "Smooth blend dissolve between clips", iconSymbol: "⧓" },
  { id: "dip_to_black", name: "Dip to Black", desc: "Cinematic fade through black", iconSymbol: "◼" },
  { id: "dip_to_white", name: "Dip to White", desc: "High-energy flash through white", iconSymbol: "◻" },
  { id: "wipe_left", name: "Wipe Left", desc: "Horizontal wipe moving left", iconSymbol: "◀|" },
  { id: "wipe_right", name: "Wipe Right", desc: "Horizontal wipe moving right", iconSymbol: "|▶" },
  { id: "slide_left", name: "Slide Left", desc: "Push slide moving left", iconSymbol: "⇠" },
  { id: "slide_right", name: "Slide Right", desc: "Push slide moving right", iconSymbol: "⇢" },
];

const timelineTools: { id: TimelineTool; icon: IconName; label: string; shortLabel: string; shortcut: string }[] = [
  { id: "select", icon: "select", label: "Selection Tool", shortLabel: "Select", shortcut: "V" },
  { id: "trackForward", icon: "trackForward", label: "Track Select Forward", shortLabel: "Track", shortcut: "A" },
  { id: "ripple", icon: "ripple", label: "Ripple Edit Tool", shortLabel: "Ripple", shortcut: "B" },
  { id: "rolling", icon: "rolling", label: "Rolling Edit Tool", shortLabel: "Rolling", shortcut: "N" },
  { id: "razor", icon: "razor", label: "Razor Tool", shortLabel: "Razor", shortcut: "C" },
  { id: "slip", icon: "slip", label: "Slip Tool", shortLabel: "Slip", shortcut: "Y" },
  { id: "slide", icon: "slide", label: "Slide Edit Tool", shortLabel: "Slide", shortcut: "U" },
  { id: "rateStretch", icon: "rateStretch", label: "Rate Stretch Tool", shortLabel: "Stretch", shortcut: "X" },
  { id: "pen", icon: "pen", label: "Keyframe Pen Tool", shortLabel: "Pen", shortcut: "P" },
  { id: "hand", icon: "hand", label: "Hand Pan Tool", shortLabel: "Hand", shortcut: "H" },
  { id: "zoom", icon: "zoomIn", label: "Zoom Tool", shortLabel: "Zoom", shortcut: "Z" },
];
const mediaFilters = [{ name: "Video, audio, and images", extensions: ["mp4", "mkv", "mov", "webm", "avi", "m4v", "mp3", "wav", "flac", "ogg", "m4a", "aac", "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"] }];
const projectFilters = [{ name: "Easy Edits Pro project", extensions: ["saturn"] }];
const keyframeOptions: { value: KeyframeProperty; label: string; defaultValue: number; min: number; max: number; step: number }[] = [
  { value: "position_x", label: "Position X", defaultValue: 0, min: -2000, max: 2000, step: 1 },
  { value: "position_y", label: "Position Y", defaultValue: 0, min: -2000, max: 2000, step: 1 },
  { value: "scale", label: "Scale", defaultValue: 1, min: 0.01, max: 10, step: 0.01 },
  { value: "rotation", label: "Rotation", defaultValue: 0, min: -360, max: 360, step: 1 },
  { value: "opacity", label: "Opacity", defaultValue: 1, min: 0, max: 1, step: 0.01 },
];

async function runCommand<T = unknown>(id: string, params: unknown = null): Promise<T> {
  return invoke<T>("execute_command", { request: { id, params } });
}

type IconName = "fullscreen" | "restore" | "close" | "media" | "add" | "undo" | "redo" | "select" | "trackForward" | "ripple" | "rolling" | "razor" | "slip" | "slide" | "rateStretch" | "link" | "unlink" | "pen" | "hand" | "snap" | "split" | "remove" | "markIn" | "markOut" | "zoomIn" | "zoomOut" | "clearMarks" | "export" | "fileNew" | "fileOpen" | "fileSave" | "import" | "help" | "window" | "title" | "transition" | "sun" | "moon" | "color" | "audio" | "effects" | "deliver" | "edit" | "check";

function Icon({ name }: { name: IconName }) {
  const paths = {
    check: <path d="m4 10 4 4 8-8" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />,
    fullscreen: <path d="M7 3H3v4m10-4h4v4M3 13v4h4m10-4v4h-4" />,
    restore: <path d="M7 3h10v10M13 7H3v10h10V7Z" />,
    close: <path d="m5 5 10 10M15 5 5 15" />,
    media: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m9 4 2 16m5-16 2 16M3 9h18M3 15h18" /></>,
    add: <path d="M12 5v14m-7-7h14" />,
    undo: <path d="M9 14 4 9l5-5M4 9h9a6 6 0 0 1 0 12h-2" />,
    redo: <path d="m15 14 5-5-5-5m5 5h-9a6 6 0 0 0 0 12h2" />,
    select: <path d="m5 3 10 9-5 .7L8 18z" />,
    trackForward: <><path d="M3 4h8v4H3zM3 10h8v4H3zM3 16h8v2H3z" /><path d="M13 10h5m-2-2 2 2-2 2" /></>,
    ripple: <><path d="m8 5-4 5 4 5M16 5l4 5-4 5M10 10h4" /><path d="M3 18h14" /></>,
    rolling: <><path d="M7 4v12m6-12v12M4 8l3 3-3 3m12-6-3 3 3 3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    razor: <><circle cx="6" cy="6" r="2" /><circle cx="6" cy="14" r="2" /><path d="m8 7 9 9M8 13l9-9" /></>,
    slip: <><path d="M3 4h14v12H3zM6 2v2m8-2v2m-8 12v2m8-2v2" /><path d="M8 10h6m-2-2 2 2-2 2" /></>,
    slide: <><rect x="6" y="5" width="8" height="10" rx="1" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M3 10H1m0 0 1.5-1.5M1 10l1.5 1.5m16.5-1.5h-2m0 0-1.5-1.5m1.5 1.5 1.5 1.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    rateStretch: <><circle cx="10" cy="10" r="7" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M10 6v4l2.5 1.5M2 10h1.5m13 0h1.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    link: <path d="M8 11a3.5 3.5 0 0 0 5 .5l2-2a3.5 3.5 0 0 0-5-5l-1 1M12 9a3.5 3.5 0 0 0-5-.5l-2 2a3.5 3.5 0 0 0 5 5l1-1" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />,
    unlink: <><path d="M13.5 8.5l1.5-1.5a3.5 3.5 0 0 0-5-5l-1 1M6.5 11.5l-1.5 1.5a3.5 3.5 0 0 0 5 5l1-1M3 3l14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    pen: <><path d="m4 14 9-9 3 3-9 9H4zM12 6l3 3M4 17h12" /></>,
    hand: <path d="M6 11V5a1.5 1.5 0 0 1 3 0v5-7a1.5 1.5 0 0 1 3 0v7-5a1.5 1.5 0 0 1 3 0v6-3a1.5 1.5 0 0 1 3 0v5c0 4-2 7-6 7h-1c-2 0-3.5-1-4.5-2.5L3 13a1.8 1.8 0 0 1 3-2z" />,
    snap: <path d="M4 3v5a6 6 0 0 0 12 0V3M4 8h12M10 14v3m-3 0h6" />,
    split: <><path d="M4 4h12v12H4zM10 4v12" /><path d="m7 8 3 2-3 2m6-4-3 2 3 2" /></>,
    remove: <><path d="M4 6h12m-10 0 1 11h6l1-11M8 6V4h4v2m-3 3v5m2-5v5" /></>,
    markIn: <path d="M6 3H4v14h2m10-14h-2m-2 0h-2" />,
    markOut: <path d="M14 3h2v14h-2M4 3h2m2 0h2" />,
    zoomIn: <><circle cx="8.5" cy="8.5" r="5.5" /><path d="m13 13 4 4M8.5 6v5m-2.5-2.5h5" /></>,
    zoomOut: <><circle cx="8.5" cy="8.5" r="5.5" /><path d="m13 13 4 4M6 8.5h5" /></>,
    clearMarks: <><circle cx="10" cy="10" r="7" /><path d="m8 8 4 4m0-4-4 4" /></>,
    export: <><path d="M10 13V3m-4 4 4-4 4 4M4 12v5h12v-5" /></>,
    fileNew: <><path d="M5 2h7l4 4v12H5zM12 2v5h4" /><path d="M10 10v6m-3-3h6" /></>,
    fileOpen: <path d="M2 5h6l2 2h8v10H2zM2 8h16" />,
    fileSave: <><path d="M3 3h12l2 2v12H3zM6 3v5h8V3M6 17v-6h8v6" /><path d="M10 12v4m-2-2h4" /></>,
    import: <><path d="M10 3v10m-4-4 4 4 4-4M3 15v3h14v-3" /></>,
    help: <><circle cx="10" cy="10" r="8" /><path d="M7.8 7.5A2.3 2.3 0 1 1 11 9.7c-.8.5-1 1-1 2m0 2.5v.1" /></>,
    window: <><rect x="2" y="3" width="16" height="14" rx="1" /><path d="M2 7h16M7 7v10" /></>,
    title: <path d="M4 5h12M10 5v11m-3 0h6" />,
    transition: <path d="M3 5h4l6 10h4M3 15h4l6-10h4" />,
    sun: <><circle cx="10" cy="10" r="4" /><path d="M10 2v2m0 12v2M2 10h2m12 0h2m-3.5-4.5 1.4-1.4M4.9 15.1l1.4-1.4m0-7.4L4.9 4.9m10.2 10.2-1.4-1.4" /></>,
    moon: <path d="M16 11.5A6.5 6.5 0 0 1 8.5 4a6.5 6.5 0 1 0 7.5 7.5z" />,
    color: <path d="M10 2a8 8 0 1 0 8 8 3 3 0 0 0-3-3h-1.5a1.5 1.5 0 0 1-1.5-1.5V5a3 3 0 0 0-3-3z" />,
    audio: <path d="M3 8v4h3l4 4V4L6 8H3zm10-2a4 4 0 0 1 0 8m2-10a7 7 0 0 1 0 12" />,
    effects: <><path d="m10 2 2 4.5L16.5 8l-4.5 1.5L10 14l-2-4.5L3.5 8l4.5-1.5z" /><path d="m4 14 1 2 2 1-2 1-1 2-1-2-2-1 2-1z" /></>,
    deliver: <><path d="M10 3v10m-4-4 4 4 4-4M3 15v3h14v-3" /></>,
    edit: <><circle cx="6" cy="6" r="2" /><circle cx="6" cy="14" r="2" /><path d="m8 7 9 9M8 13l9-9" /></>,
  };
  return <svg viewBox="0 0 20 20" aria-hidden="true">{paths[name]}</svg>;
}

function formatTime(ticks: number) {
  const seconds = Math.max(0, Math.floor(ticks / TICKS_PER_SECOND));
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  return `${String(hours).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

function formatPan(pan: number): string {
  if (Math.abs(pan) < 0.05) return "C";
  if (pan < 0) return `L ${Math.round(Math.abs(pan) * 100)}%`;
  return `R ${Math.round(pan * 100)}%`;
}

function formatDb(db: number): string {
  if (!Number.isFinite(db) || db <= -59.5) return "-∞ dB";
  return `${db > 0 ? "+" : ""}${db.toFixed(1)} dB`;
}

const COLOR_PRESETS: { id: string; label: string; icon: string; exposure: number; contrast: number; saturation: number }[] = [
  { id: "neutral", label: "Neutral Rec.709", icon: "⚪", exposure: 0, contrast: 1.0, saturation: 1.0 },
  { id: "teal_orange", label: "Teal & Orange", icon: "🎬", exposure: 0.2, contrast: 1.25, saturation: 1.35 },
  { id: "golden_hour", label: "Golden Hour", icon: "🌅", exposure: 0.3, contrast: 1.15, saturation: 1.2 },
  { id: "cool_scifi", label: "Cool Sci-Fi", icon: "❄️", exposure: -0.2, contrast: 1.3, saturation: 0.85 },
  { id: "monochrome", label: "B&W Noir", icon: "🎞️", exposure: 0.1, contrast: 1.35, saturation: 0 },
  { id: "vintage", label: "Vintage 90s", icon: "📼", exposure: 0.15, contrast: 0.9, saturation: 0.8 },
];


function App() {
  const [appStage, setAppStage] = useState<AppStage>("welcome");
  const [loadingMessage, setLoadingMessage] = useState("Preparing your editing workspace");
  const [welcomeMode, setWelcomeMode] = useState<"home" | "new-project">("home");
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(() => {
    try {
      const recent = JSON.parse(localStorage.getItem(recentProjectsKey) ?? "[]") as RecentProject[];
      return Array.isArray(recent) ? recent.filter((item) => typeof item?.name === "string" && typeof item?.path === "string").slice(0, 6) : [];
    } catch { return []; }
  });
  const [recoveryStatus, setRecoveryStatus] = useState<RecoveryStatus>({ available: false });
  const [setupProjectName, setSetupProjectName] = useState("Untitled project");
  const [setupSequenceName, setSetupSequenceName] = useState("Sequence 1");
  const [setupResolution, setSetupResolution] = useState("1920x1080");
  const [setupWidth, setSetupWidth] = useState("1920");
  const [setupHeight, setSetupHeight] = useState("1080");
  const [setupFrameRate, setSetupFrameRate] = useState("30/1");
  const [workspace, setWorkspace] = useState<Workspace>("edit");
  const [davinciWorkspace, setDavinciWorkspace] = useState<DavinciWorkspace>("edit");
  const [filmoraTab, setFilmoraTab] = useState<FilmoraTab>("media");
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    try {
      const saved = localStorage.getItem("eep-theme");
      if (saved === "light" || saved === "dark") return saved;
    } catch {}
    return "dark";
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    try {
      localStorage.setItem("eep-theme", theme);
    } catch {}
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme((t) => (t === "dark" ? "light" : "dark"));
  }, []);

  const selectDavinciWorkspace = useCallback((ws: DavinciWorkspace) => {
    setDavinciWorkspace(ws);
    if (ws === "edit") setWorkspace("edit");
    else if (ws === "color") setWorkspace("color");
    else if (ws === "effects") setWorkspace("effects");
    else if (ws === "audio") setWorkspace("audio");
  }, []);
  const [projectState, setProjectState] = useState<ProjectStatus | null>(null);
  const [selectedMediaId, setSelectedMediaId] = useState<number | null>(null);
  const [selectedClip, setSelectedClip] = useState<SelectedClip | null>(null);
  const [selectedClips, setSelectedClips] = useState<SelectedClip[]>([]);
  const [selectedTransition, setSelectedTransition] = useState<{ track_id: number; transition: Transition } | null>(null);
  const [titleDraft, setTitleDraft] = useState<TitleProperties | null>(null);
  const [playhead, setPlayhead] = useState(0);
  const [programPlaying, setProgramPlaying] = useState(false);
  const [shuttleSpeed, setShuttleSpeed] = useState(1);
  const shuttleSpeedRef = useRef(1);
  const isPlayingRef = useRef(false);
  const clockStartAudioTimeRef = useRef(0);
  const clockStartPlayheadRef = useRef(0);
  const animFrameIdRef = useRef<number | null>(null);
  const playheadRef = useRef(0);
  const deckARef = useRef<HTMLVideoElement>(null);
  const deckBRef = useRef<HTMLVideoElement>(null);
  const [deckSnapshot, setDeckSnapshot] = useState<TimelinePlaybackSnapshot | null>(null);
  const projectRef = useRef<ProjectDocument | null>(null);
  const [sourcePreviewError, setSourcePreviewError] = useState<string | null>(null);
  const [programPreviewError, setProgramPreviewError] = useState<string | null>(null);
  const [preparedPreviews, setPreparedPreviews] = useState<Record<number, string>>({});
  const [preparingPreviews, setPreparingPreviews] = useState<Set<number>>(new Set());
  const [previewPreparationErrors, setPreviewPreparationErrors] = useState<Record<number, string>>({});
  const [mediaServerPort, setMediaServerPort] = useState<number>(0);

  useEffect(() => {
    invoke<number>("get_media_server_port")
      .then((port) => {
        if (port > 0) setMediaServerPort(port);
      })
      .catch((err) => console.warn("Could not get media server port:", err));
  }, []);

  const getMediaUrl = useCallback((filePath: string) => {
    if (mediaServerPort > 0) {
      return `http://127.0.0.1:${mediaServerPort}/media?path=${encodeURIComponent(filePath)}`;
    }
    return convertFileSrc(filePath);
  }, [mediaServerPort]);

  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [jobDrawerOpen, setJobDrawerOpen] = useState(false);
  const jobDrawerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<JobInfo[]>("get_all_jobs")
      .then((data) => {
        if (Array.isArray(data)) setJobs(data);
      })
      .catch((err) => console.warn("Could not fetch jobs:", err));

    const unlisten = listen<JobInfo>("job_queue_updated", (event) => {
      const updated = event.payload;
      setJobs((prev) => {
        const index = prev.findIndex((j) => j.id === updated.id);
        if (index >= 0) {
          const next = [...prev];
          next[index] = updated;
          return next;
        }
        return [updated, ...prev];
      });
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!jobDrawerOpen) return;
    function handleClickOutside(event: globalThis.MouseEvent) {
      if (jobDrawerRef.current && !jobDrawerRef.current.contains(event.target as Node)) {
        setJobDrawerOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [jobDrawerOpen]);

  const activeJobs = useMemo(
    () => jobs.filter((j) => j.state.status === "queued" || j.state.status === "running"),
    [jobs]
  );

  const cancelJob = useCallback(async (jobId: number) => {
    try {
      await invoke("cancel_job", { id: jobId });
    } catch (err) {
      console.warn("Failed to cancel job:", err);
    }
  }, []);

  const cancelAllJobs = useCallback(async () => {
    try {
      await invoke("cancel_all_jobs");
    } catch (err) {
      console.warn("Failed to cancel all jobs:", err);
    }
  }, []);

  const clearJobHistory = useCallback(async () => {
    try {
      await invoke("clear_job_history");
      setJobs((prev) => prev.filter((j) => j.state.status === "queued" || j.state.status === "running"));
    } catch (err) {
      console.warn("Failed to clear job history:", err);
    }
  }, []);
  const [search, setSearch] = useState("");
  const [connection, setConnection] = useState("Connecting to Rust core…");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [newProjectName, setNewProjectName] = useState("Untitled project");
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [undoStatus, setUndoStatus] = useState<CommandStatus>({ enabled: false, reason: "Nothing to undo" });
  const [redoStatus, setRedoStatus] = useState<CommandStatus>({ enabled: false, reason: "Nothing to redo" });
  const [missingMediaIds, setMissingMediaIds] = useState<Set<number>>(new Set());
  const [commands, setCommands] = useState<CommandSpec[]>([]);
  const [snapEnabled, setSnapEnabled] = useState(true);
  const [timelineTool, setTimelineTool] = useState<TimelineTool>("select");
  const [timelineZoom, setTimelineZoom] = useState(72);
  const [colorDraft, setColorDraft] = useState<ColorAdjustments>({ exposure: 0, contrast: 1, saturation: 1, hue: 0 });
  const [colorWheelsDraft, setColorWheelsDraft] = useState<ColorWheelsData>(DEFAULT_COLOR_WHEELS);
  const [gainDrafts, setGainDrafts] = useState<Record<number, number>>({});
  const [panDrafts, setPanDrafts] = useState<Record<number, number>>({});
  const [masterGainDraft, setMasterGainDraft] = useState(0);
  const [vuLevels, setVuLevels] = useState<Record<number, { left: number; right: number; peakDb: number }>>({});
  const [masterVu, setMasterVu] = useState<{ left: number; right: number; peakDb: number }>({ left: 0, right: 0, peakDb: -60 });
  const [trackGr, setTrackGr] = useState<Record<number, number>>({});
  const [masterGr, setMasterGr] = useState<number>(0);
  const [selectedFairlightTrackId, setSelectedFairlightTrackId] = useState<number | null>(null);
  const [audioProcessingDrafts, setAudioProcessingDrafts] = useState<Record<number, TrackAudioProcessing>>({});
  const [encoderInfo, setEncoderInfo] = useState<HardwareEncoderInfo | null>(null);
  const [selectedExportPreset, setSelectedExportPreset] = useState<string>("source");
  const [keyframeProperty, setKeyframeProperty] = useState<KeyframeProperty>("opacity");
  const [keyframeValue, setKeyframeValue] = useState("1");
  const [exportFormat, setExportFormat] = useState<"mp4" | "webm">("mp4");
  const [exportResolution, setExportResolution] = useState("source");
  const [exportFrameRate, setExportFrameRate] = useState("source");
  const [useHardwareEncoding, setUseHardwareEncoding] = useState(true);
  const [exportFileName, setExportFileName] = useState("Main Edit");
  const [exportOutputPath, setExportOutputPath] = useState("");
  const [exportMarkedRange, setExportMarkedRange] = useState(false);
  const [rendering, setRendering] = useState(false);
  const [renderProgress, setRenderProgress] = useState<{ fractionPercent: number; phase: string } | null>(null);
  const [assetQuery, setAssetQuery] = useState("");
  const [assetType, setAssetType] = useState<"images" | "audio">("images");
  const [assetResults, setAssetResults] = useState<OpenverseAsset[]>([]);
  const [aiModel, setAiModel] = useState("llama3.2:3b");
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiReply, setAiReply] = useState("");
  const previewPreparationInFlight = useRef(new Set<number>());
  const handPan = useRef<{ x: number; scrollLeft: number } | null>(null);
  const slipDrag = useRef<{ x: number; trackId: number; clipIndex: number } | null>(null);
  const rollDrag = useRef<{ startX: number; trackId: number; leftClipIndex: number } | null>(null);
  const slideDrag = useRef<{ startX: number; trackId: number; clipIndex: number } | null>(null);
  const stretchDrag = useRef<{ startX: number; trackId: number; clipIndex: number; initialDuration: number; initialSpeed: number } | null>(null);
  const linkedClipsDrag = useRef<Array<{ trackId: number; clipIndex: number; initialStart: number }>>([]);
  const clipPointerDrag = useRef<ClipPointerDrag | null>(null);
  const suppressClipClick = useRef(false);
  const mediaPointerDrag = useRef<MediaPointerDrag | null>(null);
  const suppressMediaClick = useRef(false);
  const timelineScrub = useRef<TimelineScrub | null>(null);
  const timelineScrollerRef = useRef<HTMLDivElement>(null);
  const zoomAnchor = useRef<{ time: number; viewportX: number } | null>(null);
  const [clipDragOffset, setClipDragOffset] = useState<{ trackId: number; clipIndex: number; x: number; y: number } | null>(null);
  const [draggedMediaId, setDraggedMediaId] = useState<number | null>(null);
  const [mediaDropTargetTrackId, setMediaDropTargetTrackId] = useState<number | null>(null);

  const refreshProject = useCallback(async () => {
    const [status, undo, redo, mediaAvailability] = await Promise.all([
      runCommand<ProjectStatus>("project.status"),
      invoke<CommandStatus>("command_status", { id: "edit.undo" }),
      invoke<CommandStatus>("command_status", { id: "edit.redo" }),
      invoke<MediaAvailability[]>("check_media_availability"),
    ]);
    setProjectState(status);
    setUndoStatus(undo);
    setRedoStatus(redo);
    setMissingMediaIds(new Set(mediaAvailability.filter((item) => item.missing).map((item) => item.mediaId)));
    setConnection("Rust core connected");
    setSelectedMediaId((current) => current ?? status.document.project.media[0]?.id ?? null);
    return status;
  }, []);

  useEffect(() => {
    let active = true;
    invoke<CommandSpec[]>("get_command_catalogue").then((catalogue) => { if (active) setCommands(catalogue); }).catch(() => {});
    invoke<RecoveryStatus>("recovery_status").then((recovery) => { if (active) { setRecoveryStatus(recovery); setConnection("Rust core connected"); } })
      .catch((error: unknown) => { if (active) setConnection(`Rust core error: ${String(error)}`); });
    invoke<HardwareEncoderInfo>("detect_encoder_capabilities").then((info) => { if (active) setEncoderInfo(info); }).catch(() => {});
    return () => { active = false; };
  }, [refreshProject]);

  const project = projectState?.document.project ?? null;

  useEffect(() => {
    if (!project) return;
    audioClockMaster.updateTracksState(project.sequence.tracks);
  }, [project]);

  useEffect(() => {
    let animId: number;
    let active = true;

    const pollMeters = () => {
      if (!active) return;
      if (workspace === "audio" || davinciWorkspace === "audio" || programPlaying) {
        if (project) {
          const nextLevels: Record<number, { left: number; right: number; peakDb: number }> = {};
          const nextGr: Record<number, number> = {};
          for (const track of project.sequence.tracks) {
            nextLevels[track.id] = audioClockMaster.getTrackLevels(track.id);
            nextGr[track.id] = audioClockMaster.getTrackGainReduction(track.id);
          }
          setVuLevels(nextLevels);
          setTrackGr(nextGr);
          setMasterVu(audioClockMaster.getMasterLevels());
          setMasterGr(audioClockMaster.getMasterGainReduction());
        }
      }
      animId = requestAnimationFrame(pollMeters);
    };

    animId = requestAnimationFrame(pollMeters);
    return () => {
      active = false;
      cancelAnimationFrame(animId);
    };
  }, [workspace, davinciWorkspace, programPlaying, project]);
  const media = project?.media ?? [];
  const visibleMedia = useMemo(() => {
    const query = search.trim().toLowerCase();
    return media.filter((item) => !query || item.name.toLowerCase().includes(query) || item.path.toLowerCase().includes(query));
  }, [media, search]);
  const selectedMedia = media.find((item) => item.id === selectedMediaId) ?? null;
  const selectedTimelineClip = selectedClip
    ? project?.sequence.tracks.find((track) => track.id === selectedClip.track_id)?.clips[selectedClip.clip_index] ?? null
    : null;
  const activeClip = useMemo(() => {
    if (!project) return null;
    for (const track of project.sequence.tracks) {
      for (const clip of track.clips) {
        if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
          const clipMedia = media.find((item) => item.id === clip.media_id);
          if (clipMedia) return { track, clip, media: clipMedia };
        }
      }
    }
    return null;
  }, [media, playhead, project]);
  const timelineEndSeconds = Math.max(14, ...(project?.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => (clip.timeline_start + clip.duration) / TICKS_PER_SECOND + 2)));
  const timelineWidth = timelineEndSeconds * timelineZoom + 120;

  useEffect(() => {
    projectRef.current = projectState?.document ?? null;
  }, [projectState]);

  useEffect(() => {
    playheadRef.current = playhead;
  }, [playhead]);

  // Synchronize Dual Decks and Audio Tracks with Playhead
  const syncDecks = useCallback((currentPlayhead: number, isPlaying: boolean) => {
    const doc = projectRef.current;
    if (!doc) return;
    const snapshot = evaluateTimelinePlayback(doc, currentPlayhead, media, preparedPreviews, mediaServerPort);
    setDeckSnapshot(snapshot);

    const deckA = deckARef.current;
    const deckB = deckBRef.current;

    if (deckA) {
      if (snapshot.deckA.src) {
        if (deckA.src !== snapshot.deckA.src) {
          deckA.src = snapshot.deckA.src;
          try { deckA.currentTime = snapshot.deckA.currentTime; } catch {}
        } else if (Math.abs(deckA.currentTime - snapshot.deckA.currentTime) > 0.08) {
          try { deckA.currentTime = snapshot.deckA.currentTime; } catch {}
        }

        if (isPlaying && snapshot.deckA.visible) {
          if (deckA.paused) void deckA.play().catch(() => {});
        } else {
          if (!deckA.paused) deckA.pause();
        }
      } else {
        if (!deckA.paused) deckA.pause();
      }
    }

    if (deckB) {
      if (snapshot.deckB.src) {
        if (deckB.src !== snapshot.deckB.src) {
          deckB.src = snapshot.deckB.src;
          try { deckB.currentTime = snapshot.deckB.currentTime; } catch {}
        } else if (Math.abs(deckB.currentTime - snapshot.deckB.currentTime) > 0.08) {
          try { deckB.currentTime = snapshot.deckB.currentTime; } catch {}
        }

        if (isPlaying && snapshot.deckB.visible) {
          if (deckB.paused) void deckB.play().catch(() => {});
        } else {
          if (!deckB.paused) deckB.pause();
        }
      } else {
        if (!deckB.paused) deckB.pause();
      }
    }
  }, [media, preparedPreviews, mediaServerPort]);

  // Keep decks updated whenever playhead, preparedPreviews, or project changes when paused
  useEffect(() => {
    if (!isPlayingRef.current) {
      syncDecks(playhead, false);
    }
  }, [playhead, syncDecks]);

  // Proactively generate preview proxies for all video clips placed on the timeline
  useEffect(() => {
    if (!project) return;
    const requested = new Map<number, MediaAsset>();
    const timelineMediaIds = new Set(project.sequence.tracks.flatMap((t) => t.clips.map((c) => c.media_id)));
    for (const item of media) {
      if (item.kind === "video" && !preparedPreviews[item.id]) {
        if (item.id === selectedMediaId || timelineMediaIds.has(item.id)) {
          requested.set(item.id, item);
        }
      }
    }
    if (requested.size === 0) return;
    for (const [id, item] of requested) {
      if (previewPreparationInFlight.current.has(id)) continue;
      previewPreparationInFlight.current.add(id);
      setPreparingPreviews((current) => new Set(current).add(id));
      setPreviewPreparationErrors((current) => { const next = { ...current }; delete next[id]; return next; });
      void invoke<string>("prepare_video_preview", { path: item.path }).then((path) => {
        setPreparedPreviews((current) => ({ ...current, [id]: path }));
        setSourcePreviewError(null);
        setProgramPreviewError(null);
      }).catch((error: unknown) => {
        setPreviewPreparationErrors((current) => ({ ...current, [id]: String(error) }));
      }).finally(() => {
        previewPreparationInFlight.current.delete(id);
        setPreparingPreviews((current) => { const next = new Set(current); next.delete(id); return next; });
      });
    }
  }, [project, media, selectedMediaId, preparedPreviews]);

  function zoomTimeline(factor: number) {
    const nextZoom = Math.max(24, Math.min(240, timelineZoom * factor));
    if (nextZoom === timelineZoom) return;
    const scroller = timelineScrollerRef.current;
    if (scroller) {
      const viewportX = scroller.clientWidth / 2;
      zoomAnchor.current = { time: (scroller.scrollLeft + viewportX) / timelineZoom, viewportX };
    }
    setTimelineZoom(nextZoom);
  }

  useLayoutEffect(() => {
    const anchor = zoomAnchor.current;
    const scroller = timelineScrollerRef.current;
    if (!anchor || !scroller) return;
    scroller.scrollLeft = Math.max(0, anchor.time * timelineZoom - anchor.viewportX);
    zoomAnchor.current = null;
  }, [timelineZoom]);

  useEffect(() => {
    if (selectedTimelineClip) setColorDraft(selectedTimelineClip.color);
    if (selectedTimelineClip) {
      const item = media.find((m) => m.id === selectedTimelineClip.media_id);
      if (item?.kind === "title" && item.title) {
        setTitleDraft(item.title);
      } else {
        setTitleDraft(null);
      }
    } else {
      setTitleDraft(null);
    }
  }, [selectedTimelineClip, media]);

  useEffect(() => {
    if (!project) return;
    setGainDrafts(Object.fromEntries(project.sequence.tracks.map((track) => [track.id, track.gain_db])));
    for (const track of project.sequence.tracks) {
      audioClockMaster.updateTrackGain(track.id, track.gain_db);
    }
  }, [project]);

  useEffect(() => {
    setSourcePreviewError(null);
  }, [selectedMedia?.id, selectedMedia?.path]);

  function stopPlayback(finalTicks?: number) {
    if (animFrameIdRef.current) {
      cancelAnimationFrame(animFrameIdRef.current);
      animFrameIdRef.current = null;
    }
    isPlayingRef.current = false;
    setProgramPlaying(false);
    shuttleSpeedRef.current = 1;
    setShuttleSpeed(1);
    if (finalTicks !== undefined) {
      playheadRef.current = finalTicks;
      setPlayhead(finalTicks);
    }
    if (deckARef.current && !deckARef.current.paused) deckARef.current.pause();
    if (deckBRef.current && !deckBRef.current.paused) deckBRef.current.pause();
    syncDecks(playheadRef.current, false);
  }

  function startPlayback(speed = 1) {
    const ctx = audioClockMaster.getContext();
    if (ctx && ctx.state === "suspended") void ctx.resume();

    const seqDuration = Math.max(0, ...(project?.sequence.tracks ?? []).flatMap((t) => t.clips.map((c) => c.timeline_start + c.duration)));
    if (playheadRef.current >= seqDuration && speed > 0) {
      playheadRef.current = project?.sequence.mark_in ?? 0;
      setPlayhead(playheadRef.current);
    }

    shuttleSpeedRef.current = speed;
    setShuttleSpeed(speed);
    isPlayingRef.current = true;
    setProgramPlaying(true);
    setProgramPreviewError(null);
    clockStartAudioTimeRef.current = audioClockMaster.getCurrentTime();
    clockStartPlayheadRef.current = playheadRef.current;

    syncDecks(playheadRef.current, true);

    const tick = () => {
      if (!isPlayingRef.current) return;
      const now = audioClockMaster.getCurrentTime();
      const elapsedSec = (now - clockStartAudioTimeRef.current) * shuttleSpeedRef.current;
      const nextTicks = clockStartPlayheadRef.current + Math.round(elapsedSec * TICKS_PER_SECOND);

      const curDuration = Math.max(0, ...(projectRef.current?.project.sequence.tracks ?? []).flatMap((t) => t.clips.map((c) => c.timeline_start + c.duration)));

      if (shuttleSpeedRef.current > 0 && nextTicks >= curDuration) {
        stopPlayback(curDuration);
        return;
      } else if (shuttleSpeedRef.current < 0 && nextTicks <= 0) {
        stopPlayback(0);
        return;
      }

      playheadRef.current = nextTicks;
      setPlayhead(nextTicks);
      syncDecks(nextTicks, true);

      animFrameIdRef.current = requestAnimationFrame(tick);
    };

    if (animFrameIdRef.current) cancelAnimationFrame(animFrameIdRef.current);
    animFrameIdRef.current = requestAnimationFrame(tick);
  }

  function toggleProgramPlayback() {
    if (isPlayingRef.current) stopPlayback();
    else startPlayback(1);
  }

  function seekProgramTo(time: number) {
    const target = Math.max(0, time);
    playheadRef.current = target;
    setPlayhead(target);
    if (isPlayingRef.current) {
      clockStartAudioTimeRef.current = audioClockMaster.getCurrentTime();
      clockStartPlayheadRef.current = target;
    }
    syncDecks(target, isPlayingRef.current);
  }

  function stepProgramFrame(direction: -1 | 1) {
    if (isPlayingRef.current) stopPlayback();
    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
    seekProgramTo(playheadRef.current + direction * frameTicks);
  }

  function handleShuttleKey(key: "j" | "k" | "l") {
    if (key === "k") {
      stopPlayback();
      return;
    }
    if (key === "l") {
      if (!isPlayingRef.current) {
        startPlayback(1);
      } else if (shuttleSpeedRef.current === 1) {
        startPlayback(2);
      } else if (shuttleSpeedRef.current === 2) {
        startPlayback(4);
      } else {
        startPlayback(1);
      }
      return;
    }
    if (key === "j") {
      if (!isPlayingRef.current) {
        startPlayback(-1);
      } else if (shuttleSpeedRef.current === -1) {
        startPlayback(-2);
      } else if (shuttleSpeedRef.current === -2) {
        startPlayback(-4);
      } else {
        startPlayback(-1);
      }
      return;
    }
  }

  async function perform(id: string, params: unknown = null, success?: string): Promise<boolean> {
    setBusy(true);
    setNotice("");
    try {
      await runCommand(id, params);
      await refreshProject();
      if (success) setNotice(success);
      return true;
    } catch (error) {
      setNotice(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function saveColor(next: ColorAdjustments) {
    if (!selectedClip) return;
    setColorDraft(next);
    await perform("clip.set_color", { ...selectedClip, ...next }, "Color settings saved");
  }

  async function addEffect(kind: EffectKind) {
    if (!selectedClip) return;
    await perform("clip.add_effect", { ...selectedClip, kind }, "Effect added to clip");
  }

  async function removeEffect(effectId: number) {
    if (!selectedClip) return;
    await perform("clip.remove_effect", { ...selectedClip, effect_id: effectId }, "Effect removed from clip");
  }

  async function toggleEffect(effectId: number, enabled?: boolean) {
    if (!selectedClip) return;
    await perform("clip.toggle_effect", { ...selectedClip, effect_id: effectId, enabled }, "Effect toggled");
  }

  async function updateEffectPayload(effectId: number, payload: EffectPayload) {
    if (!selectedClip) return;
    await perform("clip.update_effect", { ...selectedClip, effect_id: effectId, payload }, "Effect updated");
  }

  async function reorderEffect(fromIndex: number, toIndex: number) {
    if (!selectedClip) return;
    await perform("clip.reorder_effects", { ...selectedClip, from_index: fromIndex, to_index: toIndex }, "Effects reordered");
  }

  async function saveTrackGain(trackId: number, gainDb: number) {
    setGainDrafts((current) => ({ ...current, [trackId]: gainDb }));
    audioClockMaster.updateTrackGain(trackId, gainDb);
    await perform("track.set_gain_db", { track_id: trackId, gain_db: gainDb }, "Track gain saved");
  }

  async function saveTrackPan(trackId: number, pan: number) {
    setPanDrafts((current) => ({ ...current, [trackId]: pan }));
    audioClockMaster.updateTrackPan(trackId, pan);
    await perform("track.set_pan", { track_id: trackId, pan }, "Track pan saved");
  }

  async function saveTrackAudioProcessing(trackId: number, params: Partial<TrackAudioProcessing>) {
    const track = project?.sequence.tracks.find((t) => t.id === trackId);
    const current = audioProcessingDrafts[trackId] ?? track?.audio_processing ?? {};
    const merged: TrackAudioProcessing = { ...current, ...params };
    setAudioProcessingDrafts((prev) => ({ ...prev, [trackId]: merged }));
    audioClockMaster.updateTrackAudioProcessing(trackId, merged);
    await perform("track.set_audio_processing", { track_id: trackId, ...merged }, "Audio DSP settings saved");
  }

  async function toggleTrackMute(track: Track) {
    const nextMuted = !track.muted;
    if (project) {
      const updatedTracks = project.sequence.tracks.map((t) => (t.id === track.id ? { ...t, muted: nextMuted } : t));
      audioClockMaster.updateTracksState(updatedTracks);
    }
    await perform("track.set_muted", { track_id: track.id, muted: nextMuted }, nextMuted ? "Track muted" : "Track unmuted");
  }

  async function toggleTrackSolo(track: Track) {
    const nextSolo = !track.solo;
    if (project) {
      const updatedTracks = project.sequence.tracks.map((t) => (t.id === track.id ? { ...t, solo: nextSolo } : t));
      audioClockMaster.updateTracksState(updatedTracks);
    }
    await perform("track.set_solo", { track_id: track.id, solo: nextSolo }, nextSolo ? "Track soloed" : "Track unsoloed");
  }

  async function toggleTrackLock(track: Track) {
    const nextLocked = !track.locked;
    await perform("track.set_locked", { track_id: track.id, locked: nextLocked }, nextLocked ? "Track locked" : "Track unlocked");
  }

  async function applyColorPreset(preset: typeof COLOR_PRESETS[number]) {
    const next: ColorAdjustments = { exposure: preset.exposure, contrast: preset.contrast, saturation: preset.saturation, hue: 0 };
    setColorDraft(next);
    if (selectedClip) {
      await saveColor(next);
      setNotice(`Applied "${preset.label}" color grade to clip`);
    } else {
      setNotice(`Selected "${preset.label}" preset. Select a clip to save it.`);
    }
  }

  const updateColorWheels = useCallback((wheels: ColorWheelsData) => {
    setColorWheelsDraft(wheels);
    const expDelta = wheels.offset.yLum * 2.0 + wheels.lift.yLum * 0.5;
    const nextContrast = Math.max(0.1, 1.0 + wheels.gamma.yLum * 0.6);
    const nextSat = Math.max(0, 1.0 + wheels.gain.yLum * 0.6);

    const chromX = wheels.gain.x + wheels.offset.x * 0.5;
    const chromY = wheels.gain.y + wheels.offset.y * 0.5;
    const hueAngle = Math.hypot(chromX, chromY) > 0.04
      ? Math.atan2(chromY, chromX) / Math.PI
      : 0;

    const next: ColorAdjustments = {
      exposure: Math.round(expDelta * 10) / 10,
      contrast: Math.round(nextContrast * 100) / 100,
      saturation: Math.round(nextSat * 100) / 100,
      hue: Math.round(hueAngle * 1000) / 1000,
    };
    setColorDraft(next);
    if (selectedClip) {
      void saveColor(next);
    }
  }, [selectedClip]);

  async function clearAllSolos() {
    if (!project) return;
    for (const track of project.sequence.tracks) {
      if (track.solo) {
        await perform("track.set_solo", { track_id: track.id, solo: false }, "Track unsoloed");
      }
    }
    const updated = project.sequence.tracks.map((t) => ({ ...t, solo: false }));
    audioClockMaster.updateTracksState(updated);
    setNotice("Cleared all solo tracks");
  }

  async function muteAllTracks() {
    if (!project) return;
    const anyUnmuted = project.sequence.tracks.some((t) => !t.muted);
    const nextMuted = anyUnmuted;
    for (const track of project.sequence.tracks) {
      await perform("track.set_muted", { track_id: track.id, muted: nextMuted }, nextMuted ? "Tracks muted" : "Tracks unmuted");
    }
    const updated = project.sequence.tracks.map((t) => ({ ...t, muted: nextMuted }));
    audioClockMaster.updateTracksState(updated);
    setNotice(nextMuted ? "Muted all tracks" : "Unmuted all tracks");
  }

  async function saveKeyframe(property: KeyframeProperty, value: number) {
    if (!selectedClip || !selectedTimelineClip) return;
    const time = Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start));
    await perform("clip.set_keyframe", { ...selectedClip, property, time, value }, "Keyframe saved");
  }

  async function ensureDiscardIsOkay() {
    if (!projectState?.is_dirty) return true;
    return confirm("This project has unsaved changes. Continue and discard them?", { title: "Unsaved project", kind: "warning" });
  }

  function rememberProject(name: string, path: string) {
    setRecentProjects((current) => {
      const next = [{ name, path }, ...current.filter((item) => item.path !== path)].slice(0, 6);
      try { localStorage.setItem(recentProjectsKey, JSON.stringify(next)); } catch { /* Recent projects are a convenience, not a save requirement. */ }
      return next;
    });
  }

  async function loadProjectFromPath(path: string) {
    const previousStage = appStage;
    setLoadingMessage("Opening project and preparing the timeline");
    setAppStage("loading");
    setNotice("");
    try {
      const result = await runCommand<{ name: string }>("file.open", { path });
      setSelectedClip(null);
      setSelectedClips([]);
      setSelectedTransition(null);
      setSelectedMediaId(null);
      setPlayhead(0);
      await refreshProject();
      rememberProject(result.name, path);
      setNotice("Project opened");
      setAppStage("editor");
    } catch (error) {
      setAppStage(previousStage === "editor" ? "editor" : "welcome");
      setNotice(String(error));
      throw error;
    }
  }

  async function openProject(path?: string) {
    if (appStage === "editor" && !(await ensureDiscardIsOkay())) return;
    const selectedPath = path ?? await open({ title: "Open Easy Edits Pro project", multiple: false, filters: projectFilters });
    if (typeof selectedPath !== "string") return;
    await loadProjectFromPath(selectedPath);
  }

  async function createProjectFromWelcome() {
    const name = setupProjectName.trim();
    const sequenceName = setupSequenceName.trim();
    const width = Number(setupWidth);
    const height = Number(setupHeight);
    const frameRate = frameRateOptions.find((option) => option.value === setupFrameRate) ?? frameRateOptions[4];
    if (!name || !sequenceName || !Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 || width > 16_384 || height > 16_384) {
      throw new Error("Enter project and timeline names and dimensions between 1 and 16,384 pixels.");
    }
    const selectedPath = await save({
      title: "Create Easy Edits Pro project",
      defaultPath: `${name.replace(/[\\/:*?"<>|]/g, "-")}.saturn`,
      filters: projectFilters,
    });
    if (!selectedPath) return;
    const path = selectedPath.toLowerCase().endsWith(".saturn") ? selectedPath : `${selectedPath}.saturn`;

    setLoadingMessage("Creating your project and setting up the timeline");
    setAppStage("loading");
    setNotice("");
    try {
      await runCommand("project.new", {
        name,
        sequence_name: sequenceName,
        width,
        height,
        frame_rate: { numerator: frameRate.numerator, denominator: frameRate.denominator },
      });
      await runCommand("file.save", { path });
      setSelectedClip(null);
      setSelectedClips([]);
      setSelectedTransition(null);
      setSelectedMediaId(null);
      setPlayhead(0);
      await refreshProject();
      rememberProject(name, path);
      setNotice("Project created");
      setWelcomeMode("home");
      setAppStage("editor");
    } catch (error) {
      setAppStage("welcome");
      setNotice(String(error));
      throw error;
    }
  }

  async function recoverProjectFromWelcome() {
    setLoadingMessage("Restoring your recovered project");
    setAppStage("loading");
    setNotice("");
    try {
      await invoke("restore_autosave");
      const status = await refreshProject();
      if (status.path) rememberProject(status.document.project.name, status.path);
      setNotice("Autosaved project recovered. Save it to keep your changes.");
      setAppStage("editor");
    } catch (error) {
      setAppStage("welcome");
      setNotice(String(error));
      throw error;
    }
  }

  async function createProject() {
    const name = newProjectName.trim();
    if (!name) return;
    setNewProjectOpen(false);
    setSelectedMediaId(null);
    setSelectedClip(null);
    setSelectedClips([]);
    setSelectedTransition(null);
    setPlayhead(0);
    await perform("project.new", { name }, "New project created");
  }

  async function saveProject() {
    if (!project) return;
    let path = projectState?.path ?? null;
    if (!path) {
      path = await save({
        title: "Save Easy Edits Pro project",
        defaultPath: `${project.name.replace(/[\\/:*?"<>|]/g, "-")}.saturn`,
        filters: projectFilters,
      });
    }
    if (!path) return;
    await perform("file.save", { path }, "Project saved");
  }

  async function chooseExportDestination() {
    if (!project || rendering) return;
    const extension = exportFormat;
    const safeName = (exportFileName.trim() || project.name).replace(/[\\/:*?"<>|]/g, "-");
    const path = await save({
      title: "Export video",
      defaultPath: exportOutputPath || `${safeName}.${extension}`,
      filters: [{ name: extension === "mp4" ? "MP4 video" : "WebM video", extensions: [extension] }],
    });
    if (path) {
      setExportOutputPath(path);
      const filename = path.split(/[\\/]/).at(-1) ?? "";
      setExportFileName(filename.replace(/\.[^.]+$/, ""));
    }
    return path;
  }

  async function exportProject() {
    if (!project || rendering) return;
    const outputPath = exportOutputPath || await chooseExportDestination();
    if (!outputPath) return;
    setExportOutputPath(outputPath);

    const resolution = exportResolution === "source"
      ? { width: project.settings.width, height: project.settings.height }
      : resolutionPresets.find((item) => item.value === exportResolution) ?? { width: project.settings.width, height: project.settings.height };
    const frameRate = exportFrameRate === "source"
      ? project.settings.frame_rate
      : (() => { const [numerator, denominator] = exportFrameRate.split("/").map(Number); return { numerator, denominator }; })();

    const channel = new Channel<{ fractionPercent: number; phase: string }>();
    channel.onmessage = (progress) => setRenderProgress(progress);
    setRendering(true);
    setRenderProgress({ fractionPercent: 0, phase: "Preparing timeline" });
    setNotice("");
    try {
      await invoke("start_export", {
        outputPath,
        useMarkRange: exportMarkedRange,
        settings: {
          width: resolution.width,
          height: resolution.height,
          frameRateNumerator: frameRate.numerator,
          frameRateDenominator: frameRate.denominator,
          useHardwareEncoding,
        },
        onProgress: channel,
      });
      setRenderProgress({ fractionPercent: 100, phase: "Complete" });
      setNotice(`Export complete: ${outputPath}`);
    } catch (error) {
      setNotice(`Export failed: ${String(error)}`);
    } finally {
      setRendering(false);
    }
  }

  async function searchAssets() {
    setBusy(true);
    setNotice("");
    try {
      const results = await invoke<OpenverseAsset[]>("search_openverse", { query: assetQuery, mediaType: assetType });
      setAssetResults(results);
      setNotice(`${results.length} openly licensed result${results.length === 1 ? "" : "s"} from Openverse`);
    } catch (error) {
      setNotice(String(error));
      throw error;
    } finally {
      setBusy(false);
    }
  }

  async function addLibraryAsset(asset: OpenverseAsset) {
    setBusy(true);
    setNotice(`Downloading ${asset.title}…`);
    try {
      const path = await invoke<string>("download_openverse_asset", { asset });
      const status = await invoke<ProjectStatus>("import_media", { paths: [path] });
      setProjectState(status);
      setSelectedMediaId(status.document.project.media.at(-1)?.id ?? null);
      setWorkspace("edit");
      setNotice(`${asset.title} added. Attribution was saved beside the downloaded asset.`);
    } catch (error) {
      setNotice(`Could not add asset: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function askAi() {
    setBusy(true);
    setNotice("Waiting for the local Ollama model…");
    try {
      const response = await invoke<string>("ask_local_ai", { prompt: aiPrompt, model: aiModel });
      setAiReply(response);
      setNotice("Response received from local Ollama");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function importMedia() {
    const choice = await open({ title: "Import media into Easy Edits Pro", multiple: true, filters: mediaFilters });
    const paths = choice === null ? [] : Array.isArray(choice) ? choice : [choice];
    if (paths.length === 0) return;
    setBusy(true);
    setNotice(`Scanning ${paths.length} file${paths.length === 1 ? "" : "s"}…`);
    try {
      const status = await invoke<ProjectStatus>("import_media", { paths });
      setProjectState(status);
      const availability = await invoke<MediaAvailability[]>("check_media_availability");
      setMissingMediaIds(new Set(availability.filter((item) => item.missing).map((item) => item.mediaId)));
      const lastItem = status.document.project.media.at(-1);
      if (lastItem) setSelectedMediaId(lastItem.id);
      setSelectedClip(null);
      setSelectedClips([]);
      setNotice(`${paths.length} media file${paths.length === 1 ? "" : "s"} imported`);
      const [undo, redo] = await Promise.all([
        invoke<CommandStatus>("command_status", { id: "edit.undo" }),
        invoke<CommandStatus>("command_status", { id: "edit.redo" }),
      ]);
      setUndoStatus(undo);
      setRedoStatus(redo);
    } catch (error) {
      setNotice(`Import failed: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function relinkMedia(mediaId: number) {
    const path = await open({ title: "Relink missing media", multiple: false, filters: mediaFilters });
    if (typeof path !== "string") return;
    const ok = await perform("project.relink_media", { media_id: mediaId, path }, "Media relinked");
    if (ok) setMissingMediaIds((current) => { const next = new Set(current); next.delete(mediaId); return next; });
  }

  async function addSelectedToTimeline() {
    if (!selectedMedia || !project) return;
    const desiredKind = selectedMedia.kind === "audio" ? "audio" : "video";
    const track = project.sequence.tracks.find((item) => item.kind === desiredKind);
    if (!track) {
      setNotice(`This project has no ${desiredKind} track`);
      return;
    }
    setBusy(true);
    try {
      const result = await runCommand<SelectedClip>("timeline.add_clip", { media_id: selectedMedia.id, track_id: track.id });
      setSelectedClip(result);
      setSelectedClips([]);
      setSelectedTransition(null);
      setPlayhead(0);
      await refreshProject();
      setNotice("Clip added to timeline");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function addTitleToTimeline() {
    if (!project) return;
    const videoTrack = project.sequence.tracks.find((item) => item.kind === "video");
    if (!videoTrack) {
      setNotice("This project has no video track");
      return;
    }
    setBusy(true);
    setNotice("");
    try {
      const result = await runCommand<{ media_id: number; track_id: number; clip_index: number }>("timeline.add_title", {
        text: "Title Text",
        subtitle: "Subtitle / Description",
        style: "lower_third",
        timeline_start: playhead,
        track_id: videoTrack.id,
      });
      await refreshProject();
      setSelectedClip({ track_id: result.track_id, clip_index: result.clip_index });
      setSelectedClips([]);
      setSelectedTransition(null);
      setNotice("Title graphic added to timeline");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveTitleProperty(props: Partial<TitleProperties>) {
    if (!selectedTimelineClip || !titleDraft) return;
    const next = { ...titleDraft, ...props };
    setTitleDraft(next);
    await perform("title.update", {
      media_id: selectedTimelineClip.media_id,
      ...props,
    }, "Title graphic updated");
  }

  async function addPresetTitle(preset: (typeof titlePresets)[number]) {
    if (!project) return;
    const videoTrack = project.sequence.tracks.find((item) => item.kind === "video");
    if (!videoTrack) {
      setNotice("This project has no video track");
      return;
    }
    setBusy(true);
    setNotice("");
    try {
      const result = await runCommand<{ media_id: number; track_id: number; clip_index: number }>("timeline.add_title", {
        text: preset.text,
        subtitle: preset.subtitle,
        style: preset.style,
        timeline_start: playhead,
        track_id: videoTrack.id,
      });
      await refreshProject();
      setSelectedClip({ track_id: result.track_id, clip_index: result.clip_index });
      setSelectedClips([]);
      setSelectedTransition(null);
      setNotice(`Added ${preset.name} at ${formatTime(playhead)}`);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function addPresetTransition(kind: TransitionKind) {
    if (!project) return;
    let targetTrackId = selectedClip?.track_id;
    let cutAt = playhead;
    if (selectedClip && selectedTimelineClip) {
      targetTrackId = selectedClip.track_id;
      const distStart = Math.abs(playhead - selectedTimelineClip.timeline_start);
      const distEnd = Math.abs(playhead - (selectedTimelineClip.timeline_start + selectedTimelineClip.duration));
      cutAt = distStart < distEnd ? selectedTimelineClip.timeline_start : selectedTimelineClip.timeline_start + selectedTimelineClip.duration;
    } else {
      const videoTrack = project.sequence.tracks.find((t) => t.kind === "video");
      if (videoTrack) targetTrackId = videoTrack.id;
    }
    if (targetTrackId == null) {
      setNotice("Select a video clip or track to apply transition");
      return;
    }
    setBusy(true);
    setNotice("");
    try {
      const res = await runCommand<{ transition_id: number; track_id: number; kind: string; cut_at: number; duration: number }>(
        "timeline.add_transition",
        {
          track_id: targetTrackId,
          cut_at: cutAt,
          duration: TICKS_PER_SECOND,
          kind,
        }
      );
      if (res.kind !== kind) {
        await runCommand("timeline.update_transition", {
          track_id: res.track_id,
          transition_id: res.transition_id,
          kind,
          duration: TICKS_PER_SECOND,
        });
      }
      await refreshProject();
      setSelectedTransition({
        track_id: res.track_id,
        transition: {
          id: res.transition_id,
          kind,
          cut_at: res.cut_at,
          duration: TICKS_PER_SECOND,
        },
      });
      setSelectedClip(null);
      setSelectedClips([]);
      setNotice(`Applied ${kind.replace(/_/g, " ")} transition at ${formatTime(cutAt)}`);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function addTransitionToTimeline() {
    return addPresetTransition("crossfade");
  }

  async function removeSelectedTransition() {
    if (!selectedTransition) return;
    setBusy(true);
    setNotice("");
    try {
      await runCommand("timeline.remove_transition", {
        track_id: selectedTransition.track_id,
        transition_id: selectedTransition.transition.id,
      });
      setSelectedTransition(null);
      await refreshProject();
      setNotice("Transition removed");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function updateTransition(kind: TransitionKind, duration: number) {
    if (!selectedTransition) return;
    setBusy(true);
    setNotice("");
    try {
      const res = await runCommand<{ transition_id: number; track_id: number; kind: string; cut_at: number; duration: number }>(
        "timeline.add_transition",
        {
          track_id: selectedTransition.track_id,
          cut_at: selectedTransition.transition.cut_at,
          duration,
          kind,
        }
      );
      await refreshProject();
      setSelectedTransition({
        track_id: res.track_id,
        transition: {
          id: res.transition_id,
          kind: res.kind as TransitionKind,
          cut_at: res.cut_at,
          duration: res.duration,
        },
      });
      setNotice(`Transition updated to ${kind.replace(/_/g, " ")}`);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  function beginMediaPointer(event: PointerEvent<HTMLButtonElement>, mediaId: number) {
    if (event.button !== 0) return;
    mediaPointerDrag.current = { pointerId: event.pointerId, mediaId, startX: event.clientX, startY: event.clientY, dragging: false };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function moveMediaPointer(event: PointerEvent<HTMLButtonElement>) {
    const drag = mediaPointerDrag.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (!drag.dragging && Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) >= 5) {
      drag.dragging = true;
      setDraggedMediaId(drag.mediaId);
    }
    if (!drag.dragging) return;
    event.preventDefault();
    const lane = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>(".track-lane");
    setMediaDropTargetTrackId(lane?.dataset.trackId ? Number(lane.dataset.trackId) : null);
  }

  function cancelMediaPointer(event: PointerEvent<HTMLButtonElement>) {
    if (mediaPointerDrag.current?.pointerId !== event.pointerId) return;
    mediaPointerDrag.current = null;
    setDraggedMediaId(null);
    setMediaDropTargetTrackId(null);
  }

  async function finishMediaPointer(event: PointerEvent<HTMLButtonElement>) {
    const drag = mediaPointerDrag.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    mediaPointerDrag.current = null;
    setDraggedMediaId(null);
    setMediaDropTargetTrackId(null);
    if (!drag.dragging) return;
    event.preventDefault();
    suppressMediaClick.current = true;
    window.setTimeout(() => { suppressMediaClick.current = false; }, 0);
    const lane = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>(".track-lane");
    const trackId = lane?.dataset.trackId ? Number(lane.dataset.trackId) : null;
    const mediaItem = media.find((item) => item.id === drag.mediaId);
    const track = project?.sequence.tracks.find((item) => item.id === trackId);
    if (!lane || !track || !mediaItem) return;
    if (track.locked) {
      setNotice(`Track "${track.name}" is locked against edits`);
      return;
    }
    const expectedKind = mediaItem.kind === "audio" ? "audio" : "video";
    if (track.kind !== expectedKind) {
      setNotice(`Drop ${expectedKind} media on a ${expectedKind} track`);
      return;
    }
    const laneBounds = lane.getBoundingClientRect();
    const seconds = Math.max(0, (event.clientX - laneBounds.left) / timelineZoom);
    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
    const rawStart = seconds * TICKS_PER_SECOND;
    const timelineStart = snapEnabled ? Math.round(rawStart / frameTicks) * frameTicks : Math.round(rawStart);
    setBusy(true);
    setNotice("");
    try {
      const result = await runCommand<SelectedClip>("timeline.add_clip", { media_id: drag.mediaId, track_id: track.id, timeline_start: timelineStart });
      setSelectedMediaId(drag.mediaId);
      setSelectedClip(result);
      setSelectedClips([]);
      await refreshProject();
      setNotice("Clip added at drop position");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function removeSelectedClip() {
    if (!selectedClip) return;
    if (selectedClips.length > 1) {
      await perform("timeline.ripple_delete_batch", { clips: selectedClips, ripple: false }, `${selectedClips.length} clips removed`);
    } else {
      await perform("timeline.remove_clip", selectedClip, "Clip removed");
    }
    setSelectedClip(null);
    setSelectedClips([]);
  }

  async function splitSelectedClip() {
    if (!selectedClip || !selectedTimelineClip) return;
    const split = await perform("timeline.split_clip", { ...selectedClip, at: playhead }, "Clip split at playhead");
    if (split) { setSelectedClip({ ...selectedClip, clip_index: selectedClip.clip_index + 1 }); setSelectedClips([]); }
  }

  async function rippleDeleteSelectedClip() {
    if (!selectedClip) return;
    if (selectedClips.length > 1) {
      await perform("timeline.ripple_delete_batch", { clips: selectedClips, ripple: true }, `${selectedClips.length} clips ripple deleted`);
    } else {
      await perform("timeline.ripple_delete", selectedClip, "Clip ripple deleted");
    }
    setSelectedClip(null);
    setSelectedClips([]);
  }

  async function linkSelectedClips() {
    if (selectedClips.length < 2) {
      setNotice("Select at least 2 clips (Shift+Click) to link them");
      return;
    }
    await perform("timeline.link_clips", { clips: selectedClips }, `${selectedClips.length} clips linked`);
  }

  async function unlinkSelectedClips() {
    const targets = selectedClips.length > 0 ? selectedClips : (selectedClip ? [selectedClip] : []);
    if (targets.length === 0) {
      setNotice("Select linked clips to unlink");
      return;
    }
    await perform("timeline.unlink_clips", { clips: targets }, "Clips unlinked");
  }

  async function handleTimelineClipClick(event: MouseEvent<HTMLDivElement>, trackId: number, clipIndex: number, clip: Clip) {
    if (suppressClipClick.current) {
      suppressClipClick.current = false;
      event.preventDefault();
      return;
    }
    const track = project?.sequence.tracks.find((item) => item.id === trackId);
    if (track?.locked && (timelineTool === "razor" || timelineTool === "ripple")) {
      setNotice(`Track "${track.name}" is locked`);
      return;
    }
    const selection = { track_id: trackId, clip_index: clipIndex };

    // Multi-select with Shift / Ctrl / Cmd
    if (event.shiftKey || event.ctrlKey || event.metaKey) {
      const exists = selectedClips.some((c) => c.track_id === trackId && c.clip_index === clipIndex);
      const next = exists
        ? selectedClips.filter((c) => !(c.track_id === trackId && c.clip_index === clipIndex))
        : [...selectedClips, selection];
      setSelectedClips(next);
      setSelectedClip(next[next.length - 1] ?? null);
      setSelectedTransition(null);
      return;
    }

    // Automatically select all clips in the same link group
    if (clip.group_id != null) {
      const linked = (project?.sequence.tracks ?? []).flatMap((t) =>
        t.clips
          .map((c, i) => ({ track_id: t.id, clip_index: i, clip: c }))
          .filter((item) => item.clip.group_id === clip.group_id)
          .map((item) => ({ track_id: item.track_id, clip_index: item.clip_index }))
      );
      setSelectedClips(linked);
      setSelectedClip(selection);
      setSelectedTransition(null);
      return;
    }

    setSelectedClip(selection);
    setSelectedTransition(null);
    if (timelineTool === "trackForward") {
      setSelectedClips((track?.clips.slice(clipIndex) ?? []).map((_, offset) => ({ track_id: trackId, clip_index: clipIndex + offset })));
      seekProgramTo(clip.timeline_start);
      return;
    }
    setSelectedClips([]);
    if (timelineTool === "hand" || timelineTool === "slip" || timelineTool === "rolling" || timelineTool === "slide" || timelineTool === "rateStretch") return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const fraction = Math.max(0, Math.min(1, (event.clientX - bounds.left) / Math.max(1, bounds.width)));
    const clickedTime = Math.round(clip.timeline_start + fraction * clip.duration);
    if (timelineTool === "razor") {
      seekProgramTo(clickedTime);
      const ok = await perform("timeline.split_clip", { ...selection, at: clickedTime }, "Clip split");
      if (ok) { setSelectedClip({ ...selection, clip_index: clipIndex + 1 }); setSelectedClips([]); }
    } else if (timelineTool === "ripple") {
      await perform("timeline.ripple_delete", selection, "Clip ripple deleted");
      setSelectedClip(null);
      setSelectedClips([]);
    } else if (timelineTool === "pen") {
      await perform("clip.set_keyframe", { ...selection, property: keyframeProperty, time: Math.max(0, Math.min(clip.duration, clickedTime - clip.timeline_start)), value: Number(keyframeValue) }, "Keyframe added");
    } else {
      seekProgramTo(clickedTime);
    }
  }

  function beginTimelineClipPointer(event: PointerEvent<HTMLDivElement>, trackId: number, clipIndex: number, clip: Clip) {
    const track = project?.sequence.tracks.find((t) => t.id === trackId);
    if (track?.locked) {
      setNotice(`Track "${track.name}" is locked`);
      return;
    }
    if (timelineTool === "slip") {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      setSelectedClip({ track_id: trackId, clip_index: clipIndex });
      setSelectedTransition(null);
      slipDrag.current = { x: event.clientX, trackId, clipIndex };
      return;
    }
    if (timelineTool === "rolling") {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      const bounds = event.currentTarget.getBoundingClientRect();
      const fraction = (event.clientX - bounds.left) / Math.max(1, bounds.width);
      let leftClipIndex = clipIndex;
      if (fraction < 0.35 && clipIndex > 0) {
        leftClipIndex = clipIndex - 1;
      }
      rollDrag.current = { startX: event.clientX, trackId, leftClipIndex };
      setSelectedClip({ track_id: trackId, clip_index: clipIndex });
      setSelectedClips([]);
      setSelectedTransition(null);
      return;
    }
    if (timelineTool === "slide") {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      slideDrag.current = { startX: event.clientX, trackId, clipIndex };
      setSelectedClip({ track_id: trackId, clip_index: clipIndex });
      setSelectedClips([]);
      setSelectedTransition(null);
      return;
    }
    if (timelineTool === "rateStretch") {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      stretchDrag.current = {
        startX: event.clientX,
        trackId,
        clipIndex,
        initialDuration: clip.duration,
        initialSpeed: clip.speed ?? 1.0,
      };
      setSelectedClip({ track_id: trackId, clip_index: clipIndex });
      setSelectedClips([]);
      setSelectedTransition(null);
      return;
    }
    if (timelineTool !== "select" || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    setSelectedClip({ track_id: trackId, clip_index: clipIndex });
    setSelectedTransition(null);

    // Track linked clips for synchronized dragging
    if (clip.group_id != null) {
      const linked = (project?.sequence.tracks ?? []).flatMap((t) =>
        t.clips
          .map((c, i) => ({ trackId: t.id, clipIndex: i, initialStart: c.timeline_start, group_id: c.group_id }))
          .filter((item) => item.group_id === clip.group_id)
          .map((item) => ({ trackId: item.trackId, clipIndex: item.clipIndex, initialStart: item.initialStart }))
      );
      linkedClipsDrag.current = linked;
      setSelectedClips(linked.map((l) => ({ track_id: l.trackId, clip_index: l.clipIndex })));
    } else {
      linkedClipsDrag.current = [];
      setSelectedClips([]);
    }

    clipPointerDrag.current = {
      pointerId: event.pointerId,
      trackId,
      clipIndex,
      startX: event.clientX,
      startY: event.clientY,
      startTime: clip.timeline_start,
      duration: clip.duration,
    };
  }

  function moveTimelineClipPointer(event: PointerEvent<HTMLDivElement>) {
    if (rollDrag.current) {
      const deltaX = event.clientX - rollDrag.current.startX;
      const deltaSec = (deltaX / timelineZoom).toFixed(2);
      setNotice(`Rolling Edit: ${deltaX >= 0 ? "+" : ""}${deltaSec}s`);
      return;
    }
    if (slideDrag.current) {
      const deltaX = event.clientX - slideDrag.current.startX;
      const deltaSec = (deltaX / timelineZoom).toFixed(2);
      setNotice(`Slide Edit: ${deltaX >= 0 ? "+" : ""}${deltaSec}s`);
      return;
    }
    if (stretchDrag.current) {
      const deltaX = event.clientX - stretchDrag.current.startX;
      const deltaTicks = (deltaX / timelineZoom) * TICKS_PER_SECOND;
      const proposedDur = Math.max(TICKS_PER_SECOND / 30, stretchDrag.current.initialDuration + deltaTicks);
      const speed = (stretchDrag.current.initialDuration * stretchDrag.current.initialSpeed) / proposedDur;
      setNotice(`Rate Stretch: ${Math.round(speed * 100)}% (${(proposedDur / TICKS_PER_SECOND).toFixed(2)}s)`);
      return;
    }
    const drag = clipPointerDrag.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const x = event.clientX - drag.startX;
    const y = event.clientY - drag.startY;
    if (Math.hypot(x, y) < 4) return;
    setClipDragOffset({ trackId: drag.trackId, clipIndex: drag.clipIndex, x, y });
  }

  function handleTimelineClipPointerCancel(event: PointerEvent<HTMLDivElement>) {
    if (clipPointerDrag.current?.pointerId === event.pointerId) clipPointerDrag.current = null;
    setClipDragOffset(null);
    slipDrag.current = null;
    rollDrag.current = null;
    slideDrag.current = null;
    stretchDrag.current = null;
    linkedClipsDrag.current = [];
  }

  async function finishSlip(event: PointerEvent<HTMLDivElement>) {
    const drag = slipDrag.current;
    if (!drag) return;
    slipDrag.current = null;
    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
    const delta = Math.round((event.clientX - drag.x) / timelineZoom * TICKS_PER_SECOND / frameTicks) * frameTicks;
    if (delta !== 0) await perform("timeline.slip_clip", { track_id: drag.trackId, clip_index: drag.clipIndex, delta }, "Clip source slipped");
  }

  async function finishTimelineClipPointer(event: PointerEvent<HTMLDivElement>) {
    if (slipDrag.current) {
      await finishSlip(event);
      return;
    }
    if (rollDrag.current) {
      const drag = rollDrag.current;
      rollDrag.current = null;
      const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
      const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
      const deltaX = event.clientX - drag.startX;
      const deltaTicks = snapEnabled
        ? Math.round((deltaX / timelineZoom * TICKS_PER_SECOND) / frameTicks) * frameTicks
        : Math.round(deltaX / timelineZoom * TICKS_PER_SECOND);
      if (deltaTicks !== 0) {
        await perform("timeline.roll_edit", { track_id: drag.trackId, left_clip_index: drag.leftClipIndex, delta: deltaTicks }, "Rolling edit applied");
      }
      return;
    }
    if (slideDrag.current) {
      const drag = slideDrag.current;
      slideDrag.current = null;
      const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
      const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
      const deltaX = event.clientX - drag.startX;
      const deltaTicks = snapEnabled
        ? Math.round((deltaX / timelineZoom * TICKS_PER_SECOND) / frameTicks) * frameTicks
        : Math.round(deltaX / timelineZoom * TICKS_PER_SECOND);
      if (deltaTicks !== 0) {
        await perform("timeline.slide_clip", { track_id: drag.trackId, clip_index: drag.clipIndex, delta: deltaTicks }, "Slide edit applied");
      }
      return;
    }
    if (stretchDrag.current) {
      const drag = stretchDrag.current;
      stretchDrag.current = null;
      const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
      const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
      const deltaX = event.clientX - drag.startX;
      const proposedDur = Math.max(frameTicks, drag.initialDuration + (deltaX / timelineZoom * TICKS_PER_SECOND));
      const newDuration = snapEnabled
        ? Math.round(proposedDur / frameTicks) * frameTicks
        : Math.round(proposedDur);
      if (newDuration !== drag.initialDuration) {
        await perform("timeline.rate_stretch", { track_id: drag.trackId, clip_index: drag.clipIndex, new_duration: newDuration }, "Rate stretch applied");
      }
      return;
    }

    const drag = clipPointerDrag.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    clipPointerDrag.current = null;
    setClipDragOffset(null);
    const deltaX = event.clientX - drag.startX;
    const deltaY = event.clientY - drag.startY;
    if (Math.hypot(deltaX, deltaY) < 4) {
      const bounds = event.currentTarget.getBoundingClientRect();
      const fraction = Math.max(0, Math.min(1, (event.clientX - bounds.left) / Math.max(1, bounds.width)));
      seekProgramTo(drag.startTime + fraction * drag.duration);
      return;
    }
    suppressClipClick.current = true;
    window.setTimeout(() => { suppressClipClick.current = false; }, 0);

    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);

    // If moving linked clips together
    if (linkedClipsDrag.current.length > 1) {
      const deltaTicks = snapEnabled
        ? Math.round((deltaX / timelineZoom * TICKS_PER_SECOND) / frameTicks) * frameTicks
        : Math.round(deltaX / timelineZoom * TICKS_PER_SECOND);
      const moves = linkedClipsDrag.current.map((item) => ({
        track_id: item.trackId,
        clip_index: item.clipIndex,
        timeline_start: Math.max(0, item.initialStart + deltaTicks),
      }));
      linkedClipsDrag.current = [];
      await perform("timeline.move_clips_batch", { moves }, "Linked clips moved");
      return;
    }

    const targetLane = Array.from(document.querySelectorAll<HTMLElement>(".track-lane")).find((lane) => {
      const bounds = lane.getBoundingClientRect();
      return event.clientY >= bounds.top && event.clientY <= bounds.bottom;
    });
    const targetTrackId = targetLane?.dataset.trackId ? Number(targetLane.dataset.trackId) : drag.trackId;
    const proposedStart = Math.max(0, drag.startTime + deltaX / timelineZoom * TICKS_PER_SECOND);
    const timelineStart = snapEnabled
      ? Math.round(proposedStart / frameTicks) * frameTicks
      : Math.round(proposedStart);
    setBusy(true);
    setNotice("");
    try {
      const result = await runCommand<SelectedClip & { moved: boolean }>("timeline.move_clip", {
        track_id: drag.trackId,
        clip_index: drag.clipIndex,
        target_track_id: targetTrackId,
        timeline_start: timelineStart,
      });
      setSelectedClip({ track_id: result.track_id, clip_index: result.clip_index });
      setSelectedClips([]);
      await refreshProject();
      setNotice(result.moved ? "Clip moved" : "Clip unchanged");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  function handleTimelinePanStart(event: PointerEvent<HTMLDivElement>) {
    if (timelineTool !== "hand") return;
    event.currentTarget.setPointerCapture(event.pointerId);
    handPan.current = { x: event.clientX, scrollLeft: event.currentTarget.scrollLeft };
  }

  function handleTimelinePanMove(event: PointerEvent<HTMLDivElement>) {
    if (timelineTool !== "hand" || !handPan.current) return;
    event.currentTarget.scrollLeft = handPan.current.scrollLeft - (event.clientX - handPan.current.x);
  }

  function handleTimelinePanEnd() { handPan.current = null; }

  async function invokeMenuCommand(id: string) {
    switch (id) {
      case "project.new":
        if (await ensureDiscardIsOkay()) { setNewProjectName("Untitled project"); setNewProjectOpen(true); }
        break;
      case "file.open": await openProject(); break;
      case "file.save": await saveProject(); break;
      case "project.add_media": await importMedia(); break;
      case "edit.undo": case "edit.redo": await perform(id); break;
      case "timeline.split_clip": await splitSelectedClip(); break;
      case "timeline.ripple_delete": await rippleDeleteSelectedClip(); break;
      case "timeline.remove_clip": await removeSelectedClip(); break;
      case "timeline.set_mark_in": await perform(id, { at: playhead }, "In point set"); break;
      case "timeline.set_mark_out": await perform(id, { at: playhead }, "Out point set"); break;
      case "timeline.clear_marks": await perform(id, null, "In/Out points cleared"); break;
      case "timeline.add_clip": await addSelectedToTimeline(); break;
      case "app.export": selectDavinciWorkspace("deliver"); break;
      case "view.toggle_snap": setSnapEnabled((enabled) => !enabled); break;
      case "view.zoom_in": zoomTimeline(1.2); break;
      case "view.zoom_out": zoomTimeline(1 / 1.2); break;
      case "view.zoom_reset": zoomTimeline(72 / timelineZoom); break;
      case "app.quit": await getCurrentWindow().close(); break;
      case "timeline.link_clips": await linkSelectedClips(); break;
      case "timeline.unlink_clips": await unlinkSelectedClips(); break;
      case "help.shortcuts": setNotice("Timeline tools: V Select · A Track Select Forward · C Razor · B Ripple · N Rolling · Y Slip · U Slide · X Rate Stretch · P Keyframe · H Hand · S Snap · Ctrl+L Link · Ctrl+Shift+L Unlink"); break;
      case "help.about": setNotice("Easy Edits Pro · Open source Linux video editor"); break;
      default:
        if (id.startsWith("workspace.")) {
          const ws = id.slice("workspace.".length);
          if (ws === "export" || ws === "deliver") selectDavinciWorkspace("deliver");
          else if (ws === "library" || ws === "media") selectDavinciWorkspace("media");
          else if (["edit", "color", "audio", "effects"].includes(ws)) selectDavinciWorkspace(ws as DavinciWorkspace);
          else setWorkspace(ws as Workspace);
        }
        break;
    }
  }

  function beginTimelineScrub(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const axis = event.currentTarget.classList.contains("ruler-lane") ? event.currentTarget : event.currentTarget.parentElement;
    if (!axis) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    timelineScrub.current = { pointerId: event.pointerId, axis };
    updateTimelineScrub(event.clientX);
  }

  function updateTimelineScrub(clientX: number) {
    const scrub = timelineScrub.current;
    if (!scrub) return;
    const bounds = scrub.axis.getBoundingClientRect();
    const raw = Math.max(0, (clientX - bounds.left) / timelineZoom * TICKS_PER_SECOND);
    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
    seekProgramTo(snapEnabled ? Math.round(raw / frameTicks) * frameTicks : Math.round(raw));
  }

  function moveTimelineScrub(event: PointerEvent<HTMLDivElement>) {
    if (timelineScrub.current?.pointerId !== event.pointerId) return;
    event.preventDefault();
    updateTimelineScrub(event.clientX);
  }

  function finishTimelineScrub(event: PointerEvent<HTMLDivElement>) {
    if (timelineScrub.current?.pointerId !== event.pointerId) return;
    updateTimelineScrub(event.clientX);
    timelineScrub.current = null;
  }

  async function toggleFullscreen() {
    try {
      const appWindow = getCurrentWindow();
      const isFullscreen = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!isFullscreen);
      setFullscreen(!isFullscreen);
    } catch (error) {
      setNotice(`Could not change window mode: ${String(error)}`);
    }
  }

  function renderInspector() {
    switch (workspace) {
      case "color": {
        const updateColorField = (field: keyof ColorAdjustments, value: number) => {
          const next = { ...colorDraft, [field]: value };
          setColorDraft(next);
          void saveColor(next);
        };
        return <><h2>Color grading</h2><p>{selectedTimelineClip ? "Adjust the selected timeline clip." : "Select a timeline clip to grade it."}</p>
          <label>Exposure <input type="range" min="-5" max="5" step="0.1" disabled={!selectedClip} value={colorDraft.exposure} onChange={(event) => setColorDraft((value) => ({ ...value, exposure: Number(event.target.value) }))} onPointerUp={(event) => updateColorField("exposure", Number(event.currentTarget.value))} onKeyUp={(event) => updateColorField("exposure", Number(event.currentTarget.value))} /></label>
          <label>Contrast <input type="range" min="0" max="2" step="0.01" disabled={!selectedClip} value={colorDraft.contrast} onChange={(event) => setColorDraft((value) => ({ ...value, contrast: Number(event.target.value) }))} onPointerUp={(event) => updateColorField("contrast", Number(event.currentTarget.value))} onKeyUp={(event) => updateColorField("contrast", Number(event.currentTarget.value))} /></label>
          <label>Saturation <input type="range" min="0" max="2" step="0.01" disabled={!selectedClip} value={colorDraft.saturation} onChange={(event) => setColorDraft((value) => ({ ...value, saturation: Number(event.target.value) }))} onPointerUp={(event) => updateColorField("saturation", Number(event.currentTarget.value))} onKeyUp={(event) => updateColorField("saturation", Number(event.currentTarget.value))} /></label>
          <small>Preview uses webview color filters. Values are saved with the project.</small></>;
      }
      case "effects": {
        const effects: EffectInstance[] = selectedTimelineClip?.effects ?? [];
        return (
          <div className="effects-inspector">
            <div className="effects-inspector-header">
              <h2>Effect Stack</h2>
              {selectedClip && (
                <div className="add-effect-control">
                  <select
                    className="add-effect-select"
                    defaultValue=""
                    onChange={(e) => {
                      if (e.target.value) {
                        void addEffect(e.target.value as EffectKind);
                        e.target.value = "";
                      }
                    }}
                  >
                    <option value="" disabled>+ Add Effect…</option>
                    <option value="color_correction">🎨 Color Correction</option>
                    <option value="gaussian_blur">💧 Gaussian Blur</option>
                    <option value="crop">✂️ Crop</option>
                    <option value="vignette">🔘 Vignette</option>
                    <option value="chroma_key">🟩 Chroma Key</option>
                  </select>
                </div>
              )}
            </div>

            {!selectedTimelineClip ? (
              <div className="effect-empty-card">
                <p>Select a clip on the timeline to inspect and edit its effect stack.</p>
              </div>
            ) : effects.length === 0 ? (
              <div className="effect-empty-card">
                <p>No effects applied to this clip.</p>
                <small>Select an effect from the dropdown above to begin compounding visual effects.</small>
              </div>
            ) : (
              <div className="effects-stack-list">
                {effects.map((eff, idx) => {
                  return (
                    <article key={eff.id} className={`effect-stack-card ${eff.enabled ? "is-active" : "is-bypassed"}`}>
                      <header className="effect-card-header">
                        <button
                          type="button"
                          className={`effect-toggle-btn ${eff.enabled ? "enabled" : "bypassed"}`}
                          title={eff.enabled ? "Bypass (Disable) Effect" : "Enable Effect"}
                          onClick={() => void toggleEffect(eff.id, !eff.enabled)}
                        >
                          {eff.enabled ? "👁" : "👁‍🗨"}
                        </button>
                        <div className="effect-card-title">
                          <strong>{eff.name}</strong>
                          <span className="effect-kind-badge">{eff.kind.replace("_", " ")}</span>
                        </div>
                        <div className="effect-card-actions">
                          <button
                            type="button"
                            className="effect-action-btn"
                            disabled={idx === 0}
                            title="Move Up"
                            onClick={() => void reorderEffect(idx, idx - 1)}
                          >
                            ▲
                          </button>
                          <button
                            type="button"
                            className="effect-action-btn"
                            disabled={idx === effects.length - 1}
                            title="Move Down"
                            onClick={() => void reorderEffect(idx, idx + 1)}
                          >
                            ▼
                          </button>
                          <button
                            type="button"
                            className="effect-action-btn delete"
                            title="Remove Effect"
                            onClick={() => void removeEffect(eff.id)}
                          >
                            ×
                          </button>
                        </div>
                      </header>

                      {eff.enabled && (
                        <div className="effect-card-body">
                          {eff.kind === "color_correction" && (() => {
                            const p = eff.params as ColorCorrectionParams;
                            return (
                              <>
                                <label>
                                  <span>Exposure ({p.exposure > 0 ? `+${p.exposure.toFixed(1)}` : p.exposure.toFixed(1)})</span>
                                  <input
                                    type="range"
                                    min="-5"
                                    max="5"
                                    step="0.1"
                                    value={p.exposure ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, exposure: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Contrast ({p.contrast.toFixed(2)})</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="2"
                                    step="0.01"
                                    value={p.contrast ?? 1}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, contrast: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Saturation ({p.saturation.toFixed(2)})</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="2"
                                    step="0.01"
                                    value={p.saturation ?? 1}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, saturation: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Hue Shift ({Math.round((p.hue ?? 0) * 180)}°)</span>
                                  <input
                                    type="range"
                                    min="-1"
                                    max="1"
                                    step="0.01"
                                    value={p.hue ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, hue: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </>
                            );
                          })()}

                          {eff.kind === "gaussian_blur" && (() => {
                            const p = eff.params as GaussianBlurParams;
                            return (
                              <label>
                                <span>Blur Radius ({p.sigma.toFixed(1)} px)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="20"
                                  step="0.1"
                                  value={p.sigma ?? 2}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "gaussian_blur",
                                    params: { sigma: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            );
                          })()}

                          {eff.kind === "crop" && (() => {
                            const p = eff.params as CropParams;
                            return (
                              <div className="crop-controls-grid">
                                <label>
                                  <span>Top ({p.top ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.top ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, top: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Bottom ({p.bottom ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.bottom ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, bottom: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Left ({p.left ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.left ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, left: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Right ({p.right ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.right ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, right: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </div>
                            );
                          })()}

                          {eff.kind === "vignette" && (() => {
                            const p = eff.params as VignetteParams;
                            return (
                              <>
                                <label>
                                  <span>Intensity ({Math.round((p.intensity ?? 0.5) * 100)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={p.intensity ?? 0.5}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "vignette",
                                      params: { ...p, intensity: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Radius ({p.radius?.toFixed(2) ?? "0.80"})</span>
                                  <input
                                    type="range"
                                    min="0.1"
                                    max="2"
                                    step="0.01"
                                    value={p.radius ?? 0.8}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "vignette",
                                      params: { ...p, radius: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Softness ({Math.round((p.softness ?? 0.5) * 100)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={p.softness ?? 0.5}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "vignette",
                                      params: { ...p, softness: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </>
                            );
                          })()}

                          {eff.kind === "chroma_key" && (() => {
                            const p = eff.params as ChromaKeyParams;
                            return (
                              <>
                                <div className="chroma-color-picker-row">
                                  <span>Key Color</span>
                                  <div className="chroma-inputs">
                                    <input
                                      type="color"
                                      value={p.color ?? "#00ff00"}
                                      onChange={(e) => void updateEffectPayload(eff.id, {
                                        kind: "chroma_key",
                                        params: { ...p, color: e.target.value },
                                      })}
                                    />
                                    <code>{p.color ?? "#00ff00"}</code>
                                  </div>
                                </div>
                                <label>
                                  <span>Tolerance ({Math.round(p.tolerance ?? 25)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="1"
                                    value={p.tolerance ?? 25}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, tolerance: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Edge Softness ({Math.round(p.softness ?? 10)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="50"
                                    step="1"
                                    value={p.softness ?? 10}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, softness: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Spill Suppression ({Math.round((p.spill_suppression ?? 0.5) * 100)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={p.spill_suppression ?? 0.5}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, spill_suppression: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </>
                            );
                          })()}
                        </div>
                      )}
                    </article>
                  );
                })}
              </div>
            )}
            <small className="effects-footer-hint">Effects stack evaluates top-to-bottom. Real-time preview is rendered with hardware WebGL &amp; CSS pipelines.</small>
          </div>
        );
      }
      case "audio": {
        const tracks = project?.sequence.tracks ?? [];
        return (
          <div className="fairlight-mixer-container">
            <header className="fairlight-mixer-header">
              <div className="mixer-header-info">
                <h2>Fairlight Audio Console</h2>
                <small>Stereo balance, pan, mute/solo matrix &amp; master bus</small>
              </div>
              <div className="mixer-header-status">
                <span className={`mixer-live-badge ${programPlaying ? "playing" : "stopped"}`}>
                  {programPlaying ? "LIVE PLAYBACK" : "STANDBY"}
                </span>
              </div>
            </header>

            <div className="fairlight-strips-scroll">
              <div className="fairlight-strips-grid">
                {tracks.map((track) => {
                  const trackVu = vuLevels[track.id] ?? { left: 0, right: 0, peakDb: -60 };
                  const currentGain = gainDrafts[track.id] ?? track.gain_db;
                  const currentPan = panDrafts[track.id] ?? track.pan ?? 0;
                  return (
                    <article
                      className={`fairlight-channel-strip ${track.muted ? "is-muted" : ""} ${track.solo ? "is-solo" : ""} ${track.locked ? "is-locked" : ""}`}
                      key={track.id}
                    >
                      <div className="strip-top">
                        <div className="strip-track-badge">
                          <span className={`strip-kind-pill ${track.kind}`}>
                            {track.kind === "audio" ? "A" : "V"}
                          </span>
                          <strong className="strip-track-name" title={track.name}>
                            {track.name}
                          </strong>
                        </div>
                        <div className="strip-meta">
                          <span className="strip-clip-count">
                            {track.clips.length} clip{track.clips.length !== 1 ? "s" : ""}
                          </span>
                        </div>
                      </div>

                      {/* Pan Section */}
                      <div className="strip-pan-section">
                        <div className="strip-pan-header">
                          <span className="strip-section-label">PAN</span>
                          <span className="strip-pan-val">{formatPan(currentPan)}</span>
                          <button
                            type="button"
                            className="strip-pan-center-btn"
                            title="Reset pan to center"
                            onClick={() => void saveTrackPan(track.id, 0)}
                          >
                            C
                          </button>
                        </div>
                        <input
                          type="range"
                          className="strip-pan-slider"
                          min="-1"
                          max="1"
                          step="0.02"
                          value={currentPan}
                          onChange={(e) => {
                            const val = Number(e.target.value);
                            setPanDrafts((prev) => ({ ...prev, [track.id]: val }));
                            audioClockMaster.updateTrackPan(track.id, val);
                          }}
                          onPointerUp={(e) => void saveTrackPan(track.id, Number(e.currentTarget.value))}
                          onKeyUp={(e) => void saveTrackPan(track.id, Number(e.currentTarget.value))}
                        />
                      </div>

                      {/* Matrix M / S / L Buttons */}
                      <div className="strip-matrix-buttons">
                        <button
                          type="button"
                          className={`strip-btn btn-mute ${track.muted ? "active" : ""}`}
                          title={track.muted ? "Unmute track" : "Mute track"}
                          onClick={() => void toggleTrackMute(track)}
                        >
                          M
                        </button>
                        <button
                          type="button"
                          className={`strip-btn btn-solo ${track.solo ? "active" : ""}`}
                          title={track.solo ? "Solo track" : "Solo track"}
                          onClick={() => void toggleTrackSolo(track)}
                        >
                          S
                        </button>
                        <button
                          type="button"
                          className={`strip-btn btn-lock ${track.locked ? "active" : ""}`}
                          title={track.locked ? "Unlock track" : "Lock track"}
                          onClick={() => void toggleTrackLock(track)}
                        >
                          {track.locked ? "🔒" : "🔓"}
                        </button>
                      </div>

                      {/* Meter + Vertical Fader Section */}
                      <div className="strip-meter-fader-row">
                        {/* Dual Stereo VU Meter */}
                        <div className="strip-vu-meter-column" title={`Peak: ${formatDb(trackVu.peakDb)}`}>
                          <div className="vu-meter-pair">
                            <div className="vu-bar-slot">
                              <div
                                className="vu-bar-fill left"
                                style={{ height: `${Math.min(100, (trackVu.left || 0) * 100)}%` }}
                              />
                            </div>
                            <div className="vu-bar-slot">
                              <div
                                className="vu-bar-fill right"
                                style={{ height: `${Math.min(100, (trackVu.right || 0) * 100)}%` }}
                              />
                            </div>
                          </div>
                          <div className="vu-db-readout">{formatDb(trackVu.peakDb)}</div>
                        </div>

                        {/* Long-throw Vertical Fader */}
                        <div className="strip-fader-column">
                          <div className="fader-scale-ticks">
                            <span>+6</span>
                            <span className="unity">0</span>
                            <span>-6</span>
                            <span>-12</span>
                            <span>-24</span>
                            <span>-48</span>
                            <span>-∞</span>
                          </div>
                          <div className="fader-track-container">
                            <input
                              type="range"
                              className="vertical-fader"
                              min="-60"
                              max="6"
                              step="0.5"
                              value={currentGain}
                              onChange={(e) => {
                                const val = Number(e.target.value);
                                setGainDrafts((prev) => ({ ...prev, [track.id]: val }));
                                audioClockMaster.updateTrackGain(track.id, val);
                              }}
                              onPointerUp={(e) => void saveTrackGain(track.id, Number(e.currentTarget.value))}
                              onKeyUp={(e) => void saveTrackGain(track.id, Number(e.currentTarget.value))}
                            />
                          </div>
                        </div>
                      </div>

                      {/* Fader Footer */}
                      <div className="strip-fader-footer">
                        <button
                          type="button"
                          className="strip-unity-btn"
                          title="Reset track gain to 0.0 dB unity"
                          onClick={() => void saveTrackGain(track.id, 0)}
                        >
                          0 dB
                        </button>
                        <span className="strip-gain-text">
                          {currentGain > 0 ? "+" : ""}{currentGain.toFixed(1)} dB
                        </span>
                      </div>
                    </article>
                  );
                })}

                {/* Master Bus Channel Strip */}
                <article className="fairlight-channel-strip master-channel-strip">
                  <div className="strip-top master-top">
                    <div className="strip-track-badge">
                      <span className="strip-kind-pill master-pill">BUS</span>
                      <strong className="strip-track-name">MASTER</strong>
                    </div>
                    <div className="strip-meta">
                      <span className="strip-clip-count">Stereo Out</span>
                    </div>
                  </div>

                  <div className="strip-pan-section master-info-section">
                    <div className="master-rate-badge">48kHz · 24-bit</div>
                    <div className="master-routing-text">Main Output</div>
                  </div>

                  <div className="strip-matrix-buttons master-bus-placeholder">
                    <span className="master-bus-label">PROGRAM</span>
                  </div>

                  {/* Meter + Vertical Fader Section */}
                  <div className="strip-meter-fader-row">
                    {/* Master VU Meter */}
                    <div className="strip-vu-meter-column master-vu" title={`Master Peak: ${formatDb(masterVu.peakDb)}`}>
                      <div className="vu-meter-pair">
                        <div className="vu-bar-slot">
                          <div
                            className="vu-bar-fill left master"
                            style={{ height: `${Math.min(100, (masterVu.left || 0) * 100)}%` }}
                          />
                        </div>
                        <div className="vu-bar-slot">
                          <div
                            className="vu-bar-fill right master"
                            style={{ height: `${Math.min(100, (masterVu.right || 0) * 100)}%` }}
                          />
                        </div>
                      </div>
                      <div className="vu-db-readout">{formatDb(masterVu.peakDb)}</div>
                    </div>

                    {/* Master Fader */}
                    <div className="strip-fader-column">
                      <div className="fader-scale-ticks">
                        <span>+6</span>
                        <span className="unity">0</span>
                        <span>-6</span>
                        <span>-12</span>
                        <span>-24</span>
                        <span>-48</span>
                        <span>-∞</span>
                      </div>
                      <div className="fader-track-container">
                        <input
                          type="range"
                          className="vertical-fader master-fader"
                          min="-60"
                          max="6"
                          step="0.5"
                          value={masterGainDraft}
                          onChange={(e) => {
                            const val = Number(e.target.value);
                            setMasterGainDraft(val);
                            audioClockMaster.setMasterGain(val);
                          }}
                        />
                      </div>
                    </div>
                  </div>

                  {/* Master Footer */}
                  <div className="strip-fader-footer">
                    <button
                      type="button"
                      className="strip-unity-btn master-unity"
                      title="Reset master gain to 0.0 dB"
                      onClick={() => {
                        setMasterGainDraft(0);
                        audioClockMaster.setMasterGain(0);
                      }}
                    >
                      0 dB
                    </button>
                    <span className="strip-gain-text master-gain-text">
                      {masterGainDraft > 0 ? "+" : ""}{masterGainDraft.toFixed(1)} dB
                    </span>
                  </div>
                </article>
              </div>
            </div>
          </div>
        );
      }
      case "keyframes": {
        const option = keyframeOptions.find((item) => item.value === keyframeProperty)!;
        const keyframes = selectedTimelineClip?.keyframes ?? [];
        return <><h2>Keyframes</h2><p>{selectedTimelineClip ? "Add or remove clip property keys at the current playhead." : "Select a timeline clip to edit its keyframes."}</p>
          <label>Property <select disabled={!selectedClip} value={keyframeProperty} onChange={(event) => { const value = event.target.value as KeyframeProperty; setKeyframeProperty(value); setKeyframeValue(String(keyframeOptions.find((item) => item.value === value)?.defaultValue ?? 0)); }}>{keyframeOptions.map((item) => <option value={item.value} key={item.value}>{item.label}</option>)}</select></label>
          <div className="control">Clip time <span>{selectedTimelineClip ? formatTime(Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start))) : "—"}</span></div>
          <label>Value <input type="number" min={option.min} max={option.max} step={option.step} disabled={!selectedClip} value={keyframeValue} onChange={(event) => setKeyframeValue(event.target.value)} /></label>
          <button className="primary" disabled={!selectedClip || !Number.isFinite(Number(keyframeValue))} onClick={() => void saveKeyframe(keyframeProperty, Number(keyframeValue))}>Set keyframe</button>
          <div className="keyframe-list">{keyframes.map((keyframe) => <div className="control" key={`${keyframe.property}-${keyframe.time}`}><span>{keyframeOptions.find((item) => item.value === keyframe.property)?.label} · {formatTime(keyframe.time)} · {keyframe.value}</span><button aria-label="Remove keyframe" onClick={() => selectedClip && void perform("clip.remove_keyframe", { ...selectedClip, property: keyframe.property, time: keyframe.time }, "Keyframe removed")}>×</button></div>)}</div>
          <small>Keyframes are stored in the project. Animated playback interpolation is not connected yet.</small></>;
      }
      case "export":
        return <><h2>Export</h2><p>Render the sequence using its project resolution and frame rate.</p>
          <div className="control">In point <span>{project?.sequence.mark_in == null ? "Not set" : formatTime(project.sequence.mark_in)}</span></div>
          <div className="control">Out point <span>{project?.sequence.mark_out == null ? "Not set" : formatTime(project.sequence.mark_out)}</span></div>
          <label className="range-export"><input type="checkbox" checked={exportMarkedRange} disabled={project?.sequence.mark_in == null || project?.sequence.mark_out == null} onChange={(event) => setExportMarkedRange(event.target.checked)} /> Export marked In/Out range</label>
          <button disabled={project?.sequence.mark_in == null && project?.sequence.mark_out == null} onClick={() => void invokeMenuCommand("timeline.clear_marks")}>Clear marks</button>
          <label>Format <select value={exportFormat} disabled={rendering} onChange={(event) => setExportFormat(event.target.value as "mp4" | "webm")}><option value="mp4">MP4 · H.264 + AAC</option><option value="webm">WebM · VP8 + Vorbis</option></select></label>
          <div className="control">Resolution <span>{project ? `${project.settings.width} × ${project.settings.height}` : "—"}</span></div>
          <div className="control">Frame rate <span>{project ? `${project.settings.frame_rate.numerator}/${project.settings.frame_rate.denominator} fps` : "—"}</span></div>
          <button className="primary" disabled={!project || rendering || !project.sequence.tracks.some((track) => track.clips.length > 0)} onClick={() => void exportProject()}>{rendering ? "Rendering…" : "Render video"}</button>
          {renderProgress && <div className="render-progress"><progress max={100} value={renderProgress.fractionPercent} /><small>{renderProgress.phase} · {renderProgress.fractionPercent}%</small></div>}
          <small>MP4 prefers an installed VA-API, Quick Sync, NVENC, or V4L2 H.264 encoder; software encoding is used when hardware is unavailable. Color controls and track gain are included; animated keyframes are not yet rendered.</small></>;
      case "library":
        return <><h2>FOSS asset library</h2><p>Search openly licensed media indexed by Openverse. Check each source page and license before publishing your work.</p>
          <label>Search <input value={assetQuery} maxLength={200} placeholder="Try nature, city, music…" onChange={(event) => setAssetQuery(event.target.value)} /></label>
          <label>Media type <select value={assetType} onChange={(event) => setAssetType(event.target.value as typeof assetType)}><option value="images">Images</option><option value="audio">Audio</option></select></label>
          <LoadingButton className="primary" disabled={busy || !assetQuery.trim()} onAction={searchAssets} pendingLabel="Searching…" successLabel="Search complete" errorLabel="Search failed">Search Openverse</LoadingButton>
          <div className="asset-results">{assetResults.map((asset) => <article className="asset-result" key={`${asset.id}-${asset.mediaUrl}`}>
            {asset.thumbnailUrl && <img src={asset.thumbnailUrl} alt="" loading="lazy" />}
            <strong>{asset.title}</strong><small>{asset.creator ? `By ${asset.creator}` : "Creator not listed"} · {asset.license ?? "Check source license"}</small>
            <div className="asset-actions"><a href={asset.sourceUrl} target="_blank" rel="noreferrer">Source and license</a><button disabled={busy} onClick={() => void addLibraryAsset(asset)}>Add to media bin</button></div>
          </article>)}</div></>;
      case "ai":
        return <><h2>Local AI assistant</h2><p>Easy Edits Pro sends prompts only to Ollama running on this computer. Install Ollama and pull a model before use.</p>
          <label>Ollama model <input value={aiModel} maxLength={100} onChange={(event) => setAiModel(event.target.value)} /></label>
          <label>Prompt <textarea rows={5} maxLength={12000} value={aiPrompt} placeholder="Ask for editing ideas, a shot order, or help planning a sequence…" onChange={(event) => setAiPrompt(event.target.value)} /></label>
          <button className="primary" disabled={busy || !aiPrompt.trim() || !aiModel.trim()} onClick={() => void askAi()}>Ask local model</button>
          {aiReply && <pre className="ai-reply">{aiReply}</pre>}
          <small>AI suggestions do not modify the timeline or project automatically.</small></>;
      default:
        if (selectedTransition) {
          const { transition, track_id } = selectedTransition;
          const durationSeconds = transition.duration / TICKS_PER_SECOND;
          return (
            <>
              <h2>Transition inspector</h2>
              <p>Style and timing for track transitions.</p>
              <div className="control">Track <span>Track {track_id}</span></div>
              <div className="control">Cut position <span>{formatTime(transition.cut_at)}</span></div>
              <label>
                Transition type
                <select
                  value={transition.kind}
                  onChange={(e) => void updateTransition(e.target.value as TransitionKind, transition.duration)}
                >
                  <option value="crossfade">Crossfade (Smooth Blend)</option>
                  <option value="dip_to_black">Dip to Black</option>
                  <option value="dip_to_white">Dip to White</option>
                  <option value="wipe_left">Wipe Left</option>
                  <option value="wipe_right">Wipe Right</option>
                  <option value="slide_left">Slide Left</option>
                  <option value="slide_right">Slide Right</option>
                </select>
              </label>
              <label>
                Duration ({durationSeconds.toFixed(2)}s)
                <input
                  type="range"
                  min="0.2"
                  max="4.0"
                  step="0.1"
                  value={durationSeconds}
                  onChange={(e) => {
                    const durTicks = Math.round(Number(e.target.value) * TICKS_PER_SECOND);
                    setSelectedTransition({
                      track_id,
                      transition: { ...transition, duration: durTicks },
                    });
                  }}
                  onPointerUp={(e) => {
                    const durTicks = Math.round(Number(e.currentTarget.value) * TICKS_PER_SECOND);
                    void updateTransition(transition.kind, durTicks);
                  }}
                />
              </label>
              <button className="danger-button" onClick={() => void removeSelectedTransition()}>Remove transition</button>
            </>
          );
        }
        if (selectedTimelineClip && selectedClip) {
          const mediaForClip = media.find((item) => item.id === selectedTimelineClip.media_id);
          if (mediaForClip?.kind === "title") {
            const currentTitle = titleDraft ?? mediaForClip.title ?? {
              text: "Title Text",
              subtitle: "",
              style: "lower_third",
              font_family: "Inter",
              font_size: 42,
              color: "#ffffff",
              bg_color: "rgba(15, 23, 42, 0.75)",
            };
            return (
              <>
                <h2>Title & Graphic inspector</h2>
                <p>Customize lower-thirds, headline cards, and overlays.</p>
                <label>
                  Headline text
                  <input
                    value={currentTitle.text}
                    onChange={(e) => setTitleDraft({ ...currentTitle, text: e.target.value })}
                    onBlur={(e) => void saveTitleProperty({ text: e.target.value })}
                    placeholder="Headline text"
                  />
                </label>
                <label>
                  Subtitle / Description
                  <input
                    value={currentTitle.subtitle ?? ""}
                    onChange={(e) => setTitleDraft({ ...currentTitle, subtitle: e.target.value })}
                    onBlur={(e) => void saveTitleProperty({ subtitle: e.target.value })}
                    placeholder="Presenter name, role, or description"
                  />
                </label>
                <label>
                  Graphic style
                  <select
                    value={currentTitle.style}
                    onChange={(e) => void saveTitleProperty({ style: e.target.value })}
                  >
                    <option value="lower_third">Lower-Third (Broadcasting)</option>
                    <option value="center_title">Centered Title (Cinematic)</option>
                    <option value="minimal_pill">Minimal Pill (Modern Streamer)</option>
                  </select>
                </label>
                <label>
                  Font size ({Math.round(currentTitle.font_size)}px)
                  <input
                    type="range"
                    min="18"
                    max="96"
                    step="2"
                    value={currentTitle.font_size}
                    onChange={(e) => setTitleDraft({ ...currentTitle, font_size: Number(e.target.value) })}
                    onPointerUp={(e) => void saveTitleProperty({ font_size: Number(e.currentTarget.value) })}
                  />
                </label>
                <label>
                  Text color
                  <input
                    type="color"
                    value={currentTitle.color || "#ffffff"}
                    onChange={(e) => {
                      setTitleDraft({ ...currentTitle, color: e.target.value });
                      void saveTitleProperty({ color: e.target.value });
                    }}
                  />
                </label>
                <div className="control">Start <span>{formatTime(selectedTimelineClip.timeline_start)}</span></div>
                <div className="control">Duration <span>{formatTime(selectedTimelineClip.duration)}</span></div>
                <button className="danger-button" onClick={removeSelectedClip}>Remove title clip</button>
              </>
            );
          }
          return (
            <>
              <h2>Clip inspector</h2>
              <p>{mediaForClip?.name ?? "Timeline clip"}</p>
              <div className="control">Start <span>{formatTime(selectedTimelineClip.timeline_start)}</span></div>
              <div className="control">Duration <span>{formatTime(selectedTimelineClip.duration)}</span></div>
              <div className="control">Source in <span>{formatTime(selectedTimelineClip.source_in)}</span></div>
              <div className="control">Speed <span>{Math.round((selectedTimelineClip.speed ?? 1.0) * 100)}% ({selectedTimelineClip.speed ?? 1.0}x)</span></div>
              <div className="control">Group Link <span>{selectedTimelineClip.group_id != null ? `Group #${selectedTimelineClip.group_id}` : "Unlinked"}</span></div>
              {selectedTimelineClip.group_id != null ? (
                <button className="secondary" onClick={() => void unlinkSelectedClips()} disabled={busy}>Unlink from group</button>
              ) : null}
              <button className="danger-button" onClick={removeSelectedClip}>Remove clip</button>
            </>
          );
        }
        return <><h2>Inspector</h2><p>Select media or a clip to inspect its properties.</p>{selectedMedia ? <><div className="control">Type <span>{selectedMedia.kind}</span></div><div className="control">Duration <span>{selectedMedia.duration ? formatTime(selectedMedia.duration) : "—"}</span></div><div className="control">Dimensions <span>{selectedMedia.width && selectedMedia.height ? `${selectedMedia.width} × ${selectedMedia.height}` : "—"}</span></div></> : <div className="control">No selection</div>}</>;
    }
  }

  const renderTitleOverlay = (titleProps: TitleProperties | undefined) => {
    if (!titleProps) return null;
    return (
      <div
        className={`title-overlay-box style-${titleProps.style || "lower_third"}`}
        style={{
          fontFamily: titleProps.font_family || "Inter",
          color: titleProps.color || "#ffffff",
          backgroundColor: titleProps.bg_color || "rgba(15, 23, 42, 0.75)",
        }}
      >
        <div className="title-overlay-accent" />
        <div className="title-overlay-text">
          <h3 style={{ fontSize: `${Math.max(14, (titleProps.font_size || 42) * 0.45)}px` }}>
            {titleProps.text}
          </h3>
          {titleProps.subtitle && <p>{titleProps.subtitle}</p>}
        </div>
      </div>
    );
  };

  const renderProgramMonitor = () => {
    const isPreparing = deckSnapshot?.deckA.mediaId ? preparingPreviews.has(deckSnapshot.deckA.mediaId) : false;
    const currentError = programPreviewError;

    return (
      <div className="program-stage">
        {deckSnapshot && (deckSnapshot.hasVideoMedia || deckSnapshot.inTransition) && (
          <div className={`program-deck-indicator ${deckSnapshot.inTransition ? "transitioning" : ""}`}>
            <span className="deck-dot" />
            <span>{deckSnapshot.inTransition ? `TRANSITION: ${(deckSnapshot.transitionKind ?? "crossfade").toUpperCase().replace("_", " ")}` : "DECK A/B LIVE"}</span>
          </div>
        )}

        <div className="program-deck-canvas">
          {/* Deck A Video Player */}
          <video
            ref={deckARef}
            className={`program-deck deck-a ${deckSnapshot?.deckA.visible ? "" : "deck-hidden"}`}
            style={deckSnapshot?.deckA.style}
            muted={!deckSnapshot?.deckA.visible}
            playsInline
            preload="auto"
            crossOrigin="anonymous"
            onCanPlay={() => setProgramPreviewError(null)}
          />
          {deckSnapshot?.deckA.vignette && deckSnapshot.deckA.visible && (
            <div
              className="deck-vignette-overlay deck-a-vignette"
              style={{
                position: "absolute",
                inset: 0,
                pointerEvents: "none",
                zIndex: 2,
                background: `radial-gradient(ellipse at center, transparent ${(deckSnapshot.deckA.vignette.radius ?? 0.8) * 40}%, rgba(0,0,0,${deckSnapshot.deckA.vignette.intensity ?? 0.5}) ${((deckSnapshot.deckA.vignette.radius ?? 0.8) * 40) + ((deckSnapshot.deckA.vignette.softness ?? 0.5) * 50)}%)`,
              }}
            />
          )}

          {/* Deck B Video Player */}
          <video
            ref={deckBRef}
            className={`program-deck deck-b ${deckSnapshot?.deckB.visible ? "" : "deck-hidden"}`}
            style={deckSnapshot?.deckB.style}
            muted={!deckSnapshot?.deckB.visible}
            playsInline
            preload="auto"
            crossOrigin="anonymous"
            onCanPlay={() => setProgramPreviewError(null)}
          />
          {deckSnapshot?.deckB.vignette && deckSnapshot.deckB.visible && (
            <div
              className="deck-vignette-overlay deck-b-vignette"
              style={{
                position: "absolute",
                inset: 0,
                pointerEvents: "none",
                zIndex: 2,
                background: `radial-gradient(ellipse at center, transparent ${(deckSnapshot.deckB.vignette.radius ?? 0.8) * 40}%, rgba(0,0,0,${deckSnapshot.deckB.vignette.intensity ?? 0.5}) ${((deckSnapshot.deckB.vignette.radius ?? 0.8) * 40) + ((deckSnapshot.deckB.vignette.softness ?? 0.5) * 50)}%)`,
              }}
            />
          )}

          {/* Title Overlay */}
          {renderTitleOverlay(deckSnapshot?.activeTitleClip?.title)}

          {/* Dedicated Audio Track Players for Multi-Track Audio */}
          {deckSnapshot?.activeAudioClips
            .filter((c) => c.track.kind === "audio")
            .map((audioItem) => (
              <audio
                key={`audio-track-${audioItem.track.id}-${audioItem.clip.media_id}`}
                src={audioItem.src}
                autoPlay={programPlaying}
                preload="auto"
                crossOrigin="anonymous"
                ref={(el) => {
                  if (el) {
                    el.volume = Math.max(0, Math.min(1, 10 ** (audioItem.gainDb / 20)));
                    if (Math.abs(el.currentTime - audioItem.currentTime) > 0.1) {
                      try { el.currentTime = audioItem.currentTime; } catch {}
                    }
                    if (programPlaying && el.paused) void el.play().catch(() => {});
                    else if (!programPlaying && !el.paused) el.pause();
                  }
                }}
              />
            ))}

          {isPreparing && (
            <div className="preview-preparing-banner" role="status">
              <span className="preview-preparing-dot" />
              <span>Generating preview proxy…</span>
            </div>
          )}

          {currentError && !isPreparing && (
            <div className="preview-error" role="alert">
              <strong>Preview notice</strong>
              <span>{currentError}</span>
            </div>
          )}
        </div>
      </div>
    );
  };

  const renderPreview = (item: MediaAsset | null, isProgram = false) => {
    if (isProgram) {
      return renderProgramMonitor();
    }
    if (!item) {
      return <div className="monitor-stage"><strong>Source monitor</strong><small>Select imported media to preview it here</small></div>;
    }
    const isPreparing = item.kind === "video" && preparingPreviews.has(item.id) && !preparedPreviews[item.id];
    const sourcePath = item.kind === "video" && preparedPreviews[item.id] ? preparedPreviews[item.id] : item.path;
    const source = getMediaUrl(sourcePath);
    const currentError = sourcePreviewError;
    const setCurrentError = setSourcePreviewError;
    const visualStyle: CSSProperties = { width: "100%", height: "100%", objectFit: "contain" };
    if (item.kind === "video") return <div className="preview-stage">
      <video
        key={`source-${item.id}-${sourcePath}`}
        src={mediaServerPort > 0 ? source : undefined}
        style={visualStyle}
        controls
        playsInline
        preload="auto"
        crossOrigin="anonymous"
        onCanPlay={() => setCurrentError(null)}
        onError={() => {
          if (previewPreparationErrors[item.id]) {
            setCurrentError(`Preview unavailable: ${previewPreparationErrors[item.id]}`);
          } else if (isPreparing) {
            // Proxy preparation is in flight, keep quiet
          } else {
            setCurrentError("This video could not be loaded. Check that the file exists and uses a supported codec.");
          }
        }}
      />
      {isPreparing && (
        <div className="preview-preparing-banner" role="status">
          <span className="preview-preparing-dot" />
          <span>Generating preview proxy…</span>
        </div>
      )}
      {currentError && !isPreparing && <div className="preview-error" role="alert"><strong>Preview notice</strong><span>{currentError}</span></div>}
    </div>;
    if (item.kind === "audio") return <div className="preview-stage audio-preview">
      <audio
        key={`source-${item.id}-${sourcePath}`}
        src={mediaServerPort > 0 ? source : undefined}
        controls
        playsInline
        preload="auto"
        crossOrigin="anonymous"
        onCanPlay={() => setCurrentError(null)}
        onError={() => setCurrentError("This audio could not be loaded. Check that the file exists and uses a supported codec.")}
      />
      {currentError && <div className="preview-error" role="alert"><strong>Preview unavailable</strong><span>{currentError}</span></div>}
    </div>;
    if (item.kind === "image") return <div className="preview-stage">
      <img src={source} alt={item.name} style={visualStyle} />
    </div>;
    if (item.kind === "title") return <div className="preview-stage title-preview-canvas">
      {renderTitleOverlay(item.title ?? titleDraft ?? undefined)}
    </div>;
    return <div className="monitor-stage"><strong>Preview unavailable</strong><small>Easy Edits Pro could not identify this media type.</small></div>;
  };

  function renderMenu(menu: string) {
    const commandItems = commands.filter((command) => command.menu[0] === menu).map((command) => ({ id: command.id, label: command.label, shortcut: command.shortcut, disabled: busy || ((command.id === "edit.undo") && !undoStatus.enabled) || ((command.id === "edit.redo") && !redoStatus.enabled) || ((["timeline.split_clip", "timeline.ripple_delete", "timeline.remove_clip"].includes(command.id)) && !selectedClip) || (command.id === "timeline.add_clip" && !selectedMedia) }));
    const extraItems: { id: string; label: string; shortcut?: string | null; disabled?: boolean }[] = menu === "File"
      ? [{ id: "app.export", label: "Export…", shortcut: null }, { id: "app.quit", label: "Quit", shortcut: "Ctrl+Q" }]
      : menu === "View"
        ? [{ id: "view.toggle_snap", label: `Snap to frame${snapEnabled ? " ✓" : ""}`, shortcut: "S" }, { id: "view.zoom_in", label: "Zoom In", shortcut: "+" }, { id: "view.zoom_out", label: "Zoom Out", shortcut: "−" }, { id: "view.zoom_reset", label: "Reset Timeline Zoom", shortcut: "0" }]
        : menu === "Window"
          ? workspaces.map((item) => ({ id: `workspace.${item.id}`, label: `${item.label}${workspace === item.id ? " ✓" : ""}`, shortcut: null }))
          : menu === "Help"
            ? [{ id: "help.shortcuts", label: "Keyboard Shortcuts", shortcut: null }, { id: "help.about", label: "About Easy Edits Pro", shortcut: null }]
            : [];
    const items = [...commandItems, ...extraItems];
    return <details className="app-menu" key={menu}><summary>{menu}</summary><div className="menu-popover">{items.map((item) => <button key={item.id} onClick={(event) => { void invokeMenuCommand(item.id).catch((error: unknown) => setNotice(String(error))); event.currentTarget.closest("details")?.removeAttribute("open"); }} disabled={item.disabled}><span>{item.label}</span>{item.shortcut && <small>{item.shortcut.replace("Ctrl", "⌘/Ctrl")}</small>}</button>)}</div></details>;
  }

  function renderProjectWelcome() {
    if (appStage === "loading") return <main className="project-loading-screen" aria-live="polite" aria-busy="true">
      <img src="/saturn-camera.png" alt="" />
      <div className="loading-orbit"><span /><span /><span /></div>
      <p className="welcome-kicker">EASY EDITS PRO</p>
      <h1>{loadingMessage}</h1>
      <p className="welcome-muted">Getting your workspace ready</p>
      <div className="loading-track"><i /></div>
    </main>;

    return <main className="welcome-shell">
      <header className="welcome-topbar">
        <div className="welcome-brand"><img src="/saturn-camera.png" alt="" /><span><strong>Easy Edits Pro</strong><small>VIDEO EDITING WORKSPACE</small></span></div>
        <div className="welcome-status"><i className={connection.includes("error") ? "offline" : "online"} />{connection.includes("error") ? "Core unavailable" : "Ready to edit"}</div>
      </header>
      <div className="welcome-layout">
        <section className="welcome-hero">
          <div className="welcome-hero-copy">
            <p className="welcome-kicker">PROJECT STUDIO <span>·</span> {new Date().getFullYear()}</p>
            <h1>Make room<br />for the <em>next cut.</em></h1>
            <p className="welcome-intro">Pick up where you left off or shape a fresh timeline, tuned to the way you shoot.</p>
          </div>
          <div className="welcome-editor-art" aria-hidden="true">
            <div className="welcome-art-toolbar"><span /><span /><span /><i /><b>PROGRAM</b><small>00:00:12:08</small></div>
            <div className="welcome-art-preview"><span className="art-sun" /><span className="art-horizon" /><span className="art-caption">A FRAME OF YOUR OWN</span></div>
            <div className="welcome-art-timeline"><div className="art-ruler"><span>00:00</span><span>00:05</span><span>00:10</span><span>00:15</span></div><div className="art-track"><b>V1</b><i /><i /></div><div className="art-track audio"><b>A1</b><i /></div><span className="art-playhead" /></div>
          </div>
          <div className="welcome-hero-foot"><span>01 <i /> Shape the sequence</span><span>02 <i /> Find your rhythm</span><span>03 <i /> Make it yours</span></div>
        </section>

        <section className="welcome-panel" aria-label="Project selection">
          {welcomeMode === "home" ? <>
            <div className="welcome-panel-heading"><p className="welcome-kicker">YOUR WORKSPACE</p><h2>Welcome back.</h2><p>Start something new or continue a project.</p></div>
            <div className="welcome-actions">
              <button className="welcome-action-card new" onClick={() => { setSetupProjectName("Untitled project"); setSetupSequenceName("Sequence 1"); setWelcomeMode("new-project"); }}>
                <span className="welcome-action-icon">＋</span><span><strong>Create a new project</strong><small>Choose a format and start editing</small></span><b aria-hidden="true">↗</b>
              </button>
              <LoadingButton className="welcome-action-card open" onAction={() => openProject()} onError={(error) => setNotice(String(error))} pendingLabel="Opening project…" successLabel="Project ready" errorLabel="Could not open project">
                <span className="welcome-action-icon folder">↗</span><span><strong>Open an existing project</strong><small>Browse for a .saturn project file</small></span><b aria-hidden="true">→</b>
              </LoadingButton>
            </div>
            {recoveryStatus.available && <div className="welcome-recovery"><span><strong>Recovered session available</strong><small>{recoveryStatus.projectName ?? "Autosaved project"} · autosave found</small></span><LoadingButton className="welcome-recover-button" onAction={recoverProjectFromWelcome} onError={(error) => setNotice(String(error))} pendingLabel="Restoring…" successLabel="Restored" errorLabel="Restore failed">Recover</LoadingButton></div>}
            <div className="welcome-recent-heading"><h3>Recent projects</h3><span>{recentProjects.length ? `${recentProjects.length} saved here` : "Ready when you are"}</span></div>
            {recentProjects.length ? <div className="welcome-recent-list">{recentProjects.map((item) => <LoadingButton key={item.path} className="welcome-recent-item" onAction={() => openProject(item.path)} onError={(error) => setNotice(String(error))} pendingLabel="Opening…" successLabel="Opened" errorLabel="Could not open">
              <span className="recent-project-icon">S</span><span className="recent-project-name"><strong>{item.name}</strong><small title={item.path}>{item.path}</small></span><b aria-hidden="true">→</b>
            </LoadingButton>)}</div> : <div className="welcome-recent-empty"><span>⌁</span><p>Your recent projects will appear here.</p></div>}
          </> : <>
            <button className="welcome-back" onClick={() => setWelcomeMode("home")}>← <span>Back to projects</span></button>
            <div className="welcome-panel-heading setup-heading"><p className="welcome-kicker">NEW PROJECT</p><h2>Set your canvas.</h2><p>Choose the timeline format for this edit.</p></div>
            <form className="project-setup-form" onSubmit={(event) => { event.preventDefault(); void createProjectFromWelcome().catch((error: unknown) => setNotice(String(error))); }}>
              <label>Project name<input autoFocus required maxLength={120} value={setupProjectName} onChange={(event) => setSetupProjectName(event.target.value)} placeholder="Untitled project" /></label>
              <label>Timeline name<input required maxLength={120} value={setupSequenceName} onChange={(event) => setSetupSequenceName(event.target.value)} placeholder="Sequence 1" /></label>
              <div className="setup-field-row"><label>Resolution<select value={setupResolution} onChange={(event) => {
                const value = event.target.value; setSetupResolution(value);
                const preset = resolutionPresets.find((item) => item.value === value);
                if (preset && value !== "custom") { setSetupWidth(String(preset.width)); setSetupHeight(String(preset.height)); }
              }}>{resolutionPresets.map((preset) => <option value={preset.value} key={preset.value}>{preset.label}</option>)}</select></label>
                <label>Frame rate<select value={setupFrameRate} onChange={(event) => setSetupFrameRate(event.target.value)}>{frameRateOptions.map((option) => <option value={option.value} key={option.value}>{option.label}</option>)}</select></label></div>
              {setupResolution === "custom" && <div className="setup-field-row custom-resolution"><label>Width<input required type="number" min="1" max="16384" value={setupWidth} onChange={(event) => setSetupWidth(event.target.value)} /></label><span>×</span><label>Height<input required type="number" min="1" max="16384" value={setupHeight} onChange={(event) => setSetupHeight(event.target.value)} /></label></div>}
              <div className="setup-summary"><span>PROJECT PROFILE</span><strong>{setupWidth} × {setupHeight}<i />{frameRateOptions.find((option) => option.value === setupFrameRate)?.label}</strong><small>These settings define playback and export for your sequence.</small></div>
              {notice && <p className="welcome-notice" role="alert">{notice}</p>}
              <div className="setup-actions"><button type="button" className="welcome-secondary-button" onClick={() => setWelcomeMode("home")}>Cancel</button><LoadingButton className="welcome-create-button" onAction={createProjectFromWelcome} onError={(error) => setNotice(String(error))} pendingLabel="Creating project…" successLabel="Project created" errorLabel="Could not create">Create project <span>→</span></LoadingButton></div>
            </form>
          </>}
          {welcomeMode === "home" && notice && <p className="welcome-notice" role="status">{notice}</p>}
          <footer className="welcome-panel-foot"><span>PROJECT FORMAT <b>.SATURN</b></span><span>LOCAL · PRIVATE · YOURS</span></footer>
        </section>
      </div>
    </main>;
  }

  function renderAudioWorkspace() {
    const tracks = project?.sequence.tracks ?? [];
    const sequenceEnd = Math.max(0, ...tracks.flatMap((t) => t.clips.map((c) => c.timeline_start + c.duration)));
    const audioTracks = tracks.filter((t) => t.kind === "audio");
    const videoTracks = tracks.filter((t) => t.kind === "video");

    const activeTrackId = selectedFairlightTrackId ?? audioTracks[0]?.id ?? tracks[0]?.id ?? 1;
    const activeTrack = tracks.find((t) => t.id === activeTrackId) ?? tracks[0];
    const activeDsp: TrackAudioProcessing = audioProcessingDrafts[activeTrackId] ?? activeTrack?.audio_processing ?? {};

    const lowGain = activeDsp.eq_low_gain ?? 0;
    const midGain = activeDsp.eq_mid_gain ?? 0;
    const highGain = activeDsp.eq_high_gain ?? 0;
    const dyLow = (lowGain / 15) * 22;
    const dyMid = (midGain / 15) * 22;
    const dyHigh = (highGain / 15) * 22;
    const eqCurvePath = `M 0 ${(35 - dyLow).toFixed(1)} C 40 ${(35 - dyLow).toFixed(1)}, 70 ${(35 - dyMid).toFixed(1)}, 120 ${(35 - dyMid).toFixed(1)} C 170 ${(35 - dyMid).toFixed(1)}, 200 ${(35 - dyHigh).toFixed(1)}, 240 ${(35 - dyHigh).toFixed(1)}`;
    const eqFillPath = `${eqCurvePath} L 240 70 L 0 70 Z`;

    const activeTrackGr = trackGr[activeTrackId] || 0;

    return (
      <main className="workspace-page audio-workspace-page">
        <header className="workspace-sub-header audio-workspace-header">
          <div className="workspace-header-title">
            <span className="workspace-eyebrow">FAIRLIGHT · AUDIO STUDIO</span>
            <h1>Fairlight Sound Suite</h1>
            <p>Multi-track mixing console, dual stereo peak VU meters, track panning, and transport sync.</p>
          </div>
          <div className="audio-header-actions">
            <div className="audio-transport-readout">
              <span className="transport-tc-label">TIMECODE</span>
              <code className="transport-tc-val">{formatTime(playhead)}</code>
            </div>
            <div className="audio-transport-buttons">
              <button
                className="transport-btn"
                onClick={() => handleShuttleKey("j")}
                title="Shuttle Reverse (J)"
              >
                ◀◀
              </button>
              <button
                className="transport-btn"
                onClick={() => stepProgramFrame(-1)}
                title="Previous Frame (Left Arrow)"
              >
                ◀|
              </button>
              <button
                className={`transport-btn play-btn ${programPlaying ? "active-toggle" : ""}`}
                onClick={() => toggleProgramPlayback()}
                title="Play / Pause (Space / K)"
              >
                {programPlaying ? "⏸ Pause" : "▶ Play"}
              </button>
              <button
                className="transport-btn"
                onClick={() => stepProgramFrame(1)}
                title="Next Frame (Right Arrow)"
              >
                |▶
              </button>
              <button
                className="transport-btn"
                onClick={() => handleShuttleKey("l")}
                title="Shuttle Forward (L)"
              >
                ▶▶
              </button>
            </div>
            <div className="audio-matrix-global-actions">
              <button
                className="btn-secondary"
                onClick={() => void clearAllSolos()}
                title="Clear all soloed tracks"
              >
                Clear Solos
              </button>
              <button
                className="btn-secondary"
                onClick={() => void muteAllTracks()}
                title="Toggle mute for all tracks"
              >
                Mute All
              </button>
              <button
                className="btn-primary-subtle"
                onClick={() => selectDavinciWorkspace("edit")}
              >
                ← Back to Edit
              </button>
            </div>
          </div>
        </header>

        <div className="audio-studio-body">
          {/* Left Monitor Column: Picture Sync + Master Bus Card */}
          <section className="audio-monitor-column">
            <div className="audio-video-sync-box">
              <div className="audio-box-title">
                <span>PICTURE SYNC MONITOR</span>
                <code>{formatTime(playhead)} / {formatTime(sequenceEnd)}</code>
              </div>
              <div className="audio-sync-preview-stage">
                {renderProgramMonitor()}
              </div>
            </div>

            <div className="audio-master-bus-card">
              <div className="master-bus-header">
                <div className="master-bus-title">
                  <strong>MASTER OUTPUT BUS</strong>
                  <span className="bus-badge">Stereo Main · 48 kHz</span>
                </div>
                <span className={`mixer-live-badge ${programPlaying ? "playing" : "stopped"}`}>
                  {programPlaying ? "LIVE" : "STANDBY"}
                </span>
              </div>

              <div className="master-bus-controls">
                {/* Dual Stereo Master VU Meter */}
                <div className="master-vu-column">
                  <div className="vu-meter-pair master-vu-pair">
                    <div className="vu-bar-slot">
                      <div
                        className="vu-bar-fill left"
                        style={{ height: `${Math.min(100, (masterVu.left || 0) * 100)}%` }}
                      />
                    </div>
                    <div className="vu-bar-slot">
                      <div
                        className="vu-bar-fill right"
                        style={{ height: `${Math.min(100, (masterVu.right || 0) * 100)}%` }}
                      />
                    </div>
                  </div>
                  <div className="vu-db-readout">{formatDb(masterVu.peakDb)}</div>
                </div>

                {/* Master Vertical Fader */}
                <div className="master-fader-column">
                  <div className="fader-scale-ticks">
                    <span>+6</span>
                    <span className="unity">0</span>
                    <span>-6</span>
                    <span>-12</span>
                    <span>-24</span>
                    <span>-48</span>
                    <span>-∞</span>
                  </div>
                  <div className="fader-track-container">
                    <input
                      type="range"
                      className="vertical-fader master-fader"
                      min="-60"
                      max="6"
                      step="0.5"
                      value={masterGainDraft}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setMasterGainDraft(val);
                        audioClockMaster.setMasterGain(val);
                      }}
                    />
                  </div>
                  <div className="strip-fader-footer">
                    <button
                      type="button"
                      className="strip-unity-btn master-unity"
                      title="Reset master gain to unity 0 dB"
                      onClick={() => {
                        setMasterGainDraft(0);
                        audioClockMaster.setMasterGain(0);
                      }}
                    >
                      0 dB
                    </button>
                    <span className="strip-gain-text master-gain-text">{formatDb(masterGainDraft)}</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Master True Peak Limiter Module */}
            <div className="audio-master-limiter-card">
              <div className="limiter-card-header">
                <div className="limiter-title">
                  <strong>TRUE PEAK LIMITER</strong>
                  <span className="limiter-badge">-0.5 dBFS CEILING</span>
                </div>
                <span className="limiter-active-pill">BRICKWALL</span>
              </div>
              <div className="limiter-card-body">
                <div className="limiter-specs">
                  <span>Ceiling: <b>-0.5 dB</b></span>
                  <span>Ratio: <b>20:1</b></span>
                  <span>Attack: <b>1.0 ms</b></span>
                </div>
                <div className="limiter-gr-container">
                  <div className="gr-label-row">
                    <span>Master GR</span>
                    <b className={masterGr < -0.1 ? "gr-active" : ""}>
                      {masterGr < -0.05 ? `${masterGr.toFixed(1)} dB` : "0.0 dB"}
                    </b>
                  </div>
                  <div className="gr-meter-bar-slot">
                    <div
                      className="gr-meter-bar-fill master"
                      style={{ width: `${Math.min(100, Math.max(0, -masterGr * 5))}%` }}
                    />
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Center Column: Dedicated Fairlight Track Audio DSP Rack */}
          <section className="audio-dsp-rack-column">
            <div className="dsp-rack-header">
              <div className="dsp-header-left">
                <span className="dsp-eyebrow">TRACK DSP PROCESSOR</span>
                <h2>{activeTrack?.name ?? "Track"} Audio Rack</h2>
              </div>
              <div className="dsp-track-picker">
                <span className="picker-label">Target:</span>
                <select
                  value={activeTrackId}
                  onChange={(e) => setSelectedFairlightTrackId(Number(e.target.value))}
                >
                  {tracks.map((t) => (
                    <option key={t.id} value={t.id}>
                      {t.kind === "audio" ? "♪ " : "▶ "} {t.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="dsp-modules-scroll">
              {/* Module 1: 4-Band Parametric Equalizer */}
              <div className={`dsp-module-card eq-module ${activeDsp.eq_enabled ? "module-enabled" : "module-bypassed"}`}>
                <div className="dsp-module-top">
                  <div className="module-title-box">
                    <span className="module-badge eq">EQ</span>
                    <strong>4-Band Parametric Equalizer</strong>
                    <small>Low Shelf · Mid Peak · High Shelf</small>
                  </div>
                  <button
                    type="button"
                    className={`dsp-power-btn ${activeDsp.eq_enabled ? "active" : ""}`}
                    onClick={() => void saveTrackAudioProcessing(activeTrackId, { eq_enabled: !activeDsp.eq_enabled })}
                  >
                    {activeDsp.eq_enabled ? "ACTIVE" : "BYPASS"}
                  </button>
                </div>

                {/* Live EQ Curve Graph */}
                <div className="eq-curve-display">
                  <svg viewBox="0 0 240 70" preserveAspectRatio="none" className="eq-curve-svg">
                    <defs>
                      <linearGradient id="eqGrad" x1="0%" y1="0%" x2="0%" y2="100%">
                        <stop offset="0%" stopColor="#06b6d4" stopOpacity="0.35" />
                        <stop offset="100%" stopColor="#06b6d4" stopOpacity="0.0" />
                      </linearGradient>
                    </defs>
                    <line x1="0" y1="35" x2="240" y2="35" stroke="#334155" strokeDasharray="3 3" strokeWidth="1" />
                    <path d={eqFillPath} fill="url(#eqGrad)" />
                    <path d={eqCurvePath} fill="none" stroke={activeDsp.eq_enabled ? "#22d3ee" : "#64748b"} strokeWidth="2.2" strokeLinecap="round" />
                    <circle cx="20" cy={35 - dyLow} r="3.5" fill="#38bdf8" />
                    <circle cx="120" cy={35 - dyMid} r="3.5" fill="#a78bfa" />
                    <circle cx="220" cy={35 - dyHigh} r="3.5" fill="#34d399" />
                  </svg>
                  <div className="eq-freq-labels">
                    <span>80 Hz (Low)</span>
                    <span>1.0 kHz (Mid)</span>
                    <span>8.0 kHz (High)</span>
                  </div>
                </div>

                <div className="eq-bands-controls">
                  <div className="eq-band-item">
                    <div className="band-header">
                      <span>Low Shelf (80 Hz)</span>
                      <b>{lowGain > 0 ? `+${lowGain.toFixed(1)}` : lowGain.toFixed(1)} dB</b>
                    </div>
                    <input
                      type="range"
                      min="-15"
                      max="15"
                      step="0.5"
                      value={lowGain}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_low_gain: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_low_gain: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_low_gain: Number(e.currentTarget.value) })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_low_gain: Number(e.currentTarget.value) })}
                    />
                  </div>

                  <div className="eq-band-item">
                    <div className="band-header">
                      <span>Mid Bell (1 kHz)</span>
                      <b>{midGain > 0 ? `+${midGain.toFixed(1)}` : midGain.toFixed(1)} dB</b>
                    </div>
                    <input
                      type="range"
                      min="-15"
                      max="15"
                      step="0.5"
                      value={midGain}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_mid_gain: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_mid_gain: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_mid_gain: Number(e.currentTarget.value) })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_mid_gain: Number(e.currentTarget.value) })}
                    />
                  </div>

                  <div className="eq-band-item">
                    <div className="band-header">
                      <span>High Shelf (8 kHz)</span>
                      <b>{highGain > 0 ? `+${highGain.toFixed(1)}` : highGain.toFixed(1)} dB</b>
                    </div>
                    <input
                      type="range"
                      min="-15"
                      max="15"
                      step="0.5"
                      value={highGain}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, eq_high_gain: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, eq_high_gain: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_high_gain: Number(e.currentTarget.value) })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { eq_high_gain: Number(e.currentTarget.value) })}
                    />
                  </div>
                </div>
              </div>

              {/* Module 2: Dynamics Compressor */}
              <div className={`dsp-module-card comp-module ${activeDsp.comp_enabled ? "module-enabled" : "module-bypassed"}`}>
                <div className="dsp-module-top">
                  <div className="module-title-box">
                    <span className="module-badge comp">DYN</span>
                    <strong>Dynamics Compressor</strong>
                    <small>Threshold · Ratio · Attack · Release</small>
                  </div>
                  <button
                    type="button"
                    className={`dsp-power-btn comp ${activeDsp.comp_enabled ? "active" : ""}`}
                    onClick={() => void saveTrackAudioProcessing(activeTrackId, { comp_enabled: !activeDsp.comp_enabled })}
                  >
                    {activeDsp.comp_enabled ? "ACTIVE" : "BYPASS"}
                  </button>
                </div>

                {/* Real-Time Gain Reduction (GR) Meter */}
                <div className="comp-gr-meter-row">
                  <div className="gr-meta">
                    <span className="gr-title">GAIN REDUCTION (GR)</span>
                    <b className={activeTrackGr < -0.1 ? "gr-active" : ""}>
                      {activeTrackGr < -0.05 ? `${activeTrackGr.toFixed(1)} dB` : "0.0 dB"}
                    </b>
                  </div>
                  <div className="gr-meter-bar-slot">
                    <div
                      className="gr-meter-bar-fill"
                      style={{ width: `${Math.min(100, Math.max(0, -activeTrackGr * 4.16))}%` }}
                    />
                  </div>
                  <div className="gr-scale-marks">
                    <span>0</span>
                    <span>-6</span>
                    <span>-12</span>
                    <span>-18</span>
                    <span>-24 dB</span>
                  </div>
                </div>

                <div className="comp-params-grid">
                  <div className="comp-param-item">
                    <div className="param-label-row">
                      <span>Threshold</span>
                      <b>{(activeDsp.comp_threshold ?? -24).toFixed(0)} dB</b>
                    </div>
                    <input
                      type="range"
                      min="-60"
                      max="0"
                      step="1"
                      value={activeDsp.comp_threshold ?? -24}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_threshold: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_threshold: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_threshold: Number(e.currentTarget.value) })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_threshold: Number(e.currentTarget.value) })}
                    />
                  </div>

                  <div className="comp-param-item">
                    <div className="param-label-row">
                      <span>Ratio</span>
                      <b>{(activeDsp.comp_ratio ?? 4).toFixed(1)}:1</b>
                    </div>
                    <input
                      type="range"
                      min="1"
                      max="20"
                      step="0.5"
                      value={activeDsp.comp_ratio ?? 4}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_ratio: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_ratio: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_ratio: Number(e.currentTarget.value) })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_ratio: Number(e.currentTarget.value) })}
                    />
                  </div>

                  <div className="comp-param-item">
                    <div className="param-label-row">
                      <span>Attack</span>
                      <b>{Math.round((activeDsp.comp_attack ?? 0.02) * 1000)} ms</b>
                    </div>
                    <input
                      type="range"
                      min="1"
                      max="200"
                      step="1"
                      value={Math.round((activeDsp.comp_attack ?? 0.02) * 1000)}
                      onChange={(e) => {
                        const val = Number(e.target.value) / 1000;
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_attack: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_attack: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_attack: Number(e.currentTarget.value) / 1000 })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_attack: Number(e.currentTarget.value) / 1000 })}
                    />
                  </div>

                  <div className="comp-param-item">
                    <div className="param-label-row">
                      <span>Release</span>
                      <b>{Math.round((activeDsp.comp_release ?? 0.25) * 1000)} ms</b>
                    </div>
                    <input
                      type="range"
                      min="20"
                      max="1000"
                      step="10"
                      value={Math.round((activeDsp.comp_release ?? 0.25) * 1000)}
                      onChange={(e) => {
                        const val = Number(e.target.value) / 1000;
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, comp_release: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, comp_release: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_release: Number(e.currentTarget.value) / 1000 })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { comp_release: Number(e.currentTarget.value) / 1000 })}
                    />
                  </div>
                </div>
              </div>

              {/* Module 3: Vocal De-Noise & Clean-Up */}
              <div className={`dsp-module-card denoise-module ${activeDsp.denoise_enabled ? "module-enabled" : "module-bypassed"}`}>
                <div className="dsp-module-top">
                  <div className="module-title-box">
                    <span className="module-badge denoise">NR</span>
                    <strong>Vocal De-Noise &amp; Dialogue Isolation</strong>
                    <small>80 Hz High-Pass Sub Cut · Real-Time Spectral Clean</small>
                  </div>
                  <button
                    type="button"
                    className={`dsp-power-btn denoise ${activeDsp.denoise_enabled ? "active" : ""}`}
                    onClick={() => void saveTrackAudioProcessing(activeTrackId, { denoise_enabled: !activeDsp.denoise_enabled })}
                  >
                    {activeDsp.denoise_enabled ? "ACTIVE" : "BYPASS"}
                  </button>
                </div>

                <div className="denoise-controls-row">
                  <div className="denoise-rumble-badge">
                    <span className="rumble-icon">⚡</span>
                    <div className="rumble-text">
                      <strong>80 Hz High-Pass Rumble Filter</strong>
                      <small>Removes low-end mic bumps &amp; sub-bass rumble</small>
                    </div>
                    <span className={`rumble-status ${activeDsp.denoise_enabled ? "engaged" : "off"}`}>
                      {activeDsp.denoise_enabled ? "ENGAGED" : "OFF"}
                    </span>
                  </div>

                  <div className="denoise-amount-slider-item">
                    <div className="slider-header-row">
                      <span>Noise Suppression Intensity</span>
                      <b>{Math.round((activeDsp.denoise_amount ?? 0.5) * 100)}%</b>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      disabled={!activeDsp.denoise_enabled}
                      value={activeDsp.denoise_amount ?? 0.5}
                      onChange={(e) => {
                        const val = Number(e.target.value);
                        setAudioProcessingDrafts((prev) => ({ ...prev, [activeTrackId]: { ...activeDsp, denoise_amount: val } }));
                        audioClockMaster.updateTrackAudioProcessing(activeTrackId, { ...activeDsp, denoise_amount: val });
                      }}
                      onPointerUp={(e) => void saveTrackAudioProcessing(activeTrackId, { denoise_amount: Number(e.currentTarget.value) })}
                      onKeyUp={(e) => void saveTrackAudioProcessing(activeTrackId, { denoise_amount: Number(e.currentTarget.value) })}
                    />
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Right Column: Full-Height Channel Strip Mixer Rack */}
          <section className="audio-mixer-rack-column">
            <div className="mixer-rack-toolbar">
              <div className="mixer-rack-info">
                <strong>CHANNEL STRIP RACK</strong>
                <small>{tracks.length} Tracks ({audioTracks.length} Audio, {videoTracks.length} Video)</small>
              </div>
            </div>

            <div className="mixer-strips-scroll-area">
              <div className="fairlight-strips-grid">
                {tracks.map((track) => {
                  const trackVu = vuLevels[track.id] ?? { left: 0, right: 0, peakDb: -60 };
                  const currentGain = gainDrafts[track.id] ?? track.gain_db;
                  const currentPan = panDrafts[track.id] ?? track.pan ?? 0;
                  return (
                    <article
                      className={`fairlight-channel-strip ${track.id === activeTrackId ? "is-selected" : ""} ${track.muted ? "is-muted" : ""} ${track.solo ? "is-solo" : ""} ${track.locked ? "is-locked" : ""}`}
                      key={track.id}
                      onClick={() => setSelectedFairlightTrackId(track.id)}
                    >
                      <div className="strip-top">
                        <div className="strip-track-badge">
                          <span className={`strip-kind-pill ${track.kind}`}>
                            {track.kind === "audio" ? "A" : "V"}
                          </span>
                          <strong className="strip-track-name" title={track.name}>
                            {track.name}
                          </strong>
                        </div>
                        <div className="strip-meta">
                          <span className="strip-clip-count">
                            {track.clips.length} clip{track.clips.length !== 1 ? "s" : ""}
                          </span>
                        </div>
                      </div>

                      {/* Quick DSP Badges Row */}
                      <div className="strip-dsp-badges">
                        <button
                          type="button"
                          className={`strip-dsp-btn ${track.audio_processing?.eq_enabled ? "active" : ""}`}
                          title={`EQ: ${track.audio_processing?.eq_enabled ? "ON" : "OFF"}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            void saveTrackAudioProcessing(track.id, { eq_enabled: !track.audio_processing?.eq_enabled });
                          }}
                        >
                          EQ
                        </button>
                        <button
                          type="button"
                          className={`strip-dsp-btn comp ${track.audio_processing?.comp_enabled ? "active" : ""}`}
                          title={`Dynamics: ${track.audio_processing?.comp_enabled ? "ON" : "OFF"}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            void saveTrackAudioProcessing(track.id, { comp_enabled: !track.audio_processing?.comp_enabled });
                          }}
                        >
                          DYN
                        </button>
                        <button
                          type="button"
                          className={`strip-dsp-btn denoise ${track.audio_processing?.denoise_enabled ? "active" : ""}`}
                          title={`Noise Reduction: ${track.audio_processing?.denoise_enabled ? "ON" : "OFF"}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            void saveTrackAudioProcessing(track.id, { denoise_enabled: !track.audio_processing?.denoise_enabled });
                          }}
                        >
                          NR
                        </button>
                      </div>

                      {/* Pan Section */}
                      <div className="strip-pan-section">
                        <div className="strip-pan-header">
                          <span className="strip-section-label">PAN</span>
                          <span className="strip-pan-val">{formatPan(currentPan)}</span>
                          <button
                            type="button"
                            className="strip-pan-center-btn"
                            title="Reset pan to center"
                            onClick={() => void saveTrackPan(track.id, 0)}
                          >
                            C
                          </button>
                        </div>
                        <input
                          type="range"
                          className="strip-pan-slider"
                          min="-1"
                          max="1"
                          step="0.02"
                          value={currentPan}
                          onChange={(e) => {
                            const val = Number(e.target.value);
                            setPanDrafts((prev) => ({ ...prev, [track.id]: val }));
                            audioClockMaster.updateTrackPan(track.id, val);
                          }}
                          onPointerUp={(e) => void saveTrackPan(track.id, Number(e.currentTarget.value))}
                          onKeyUp={(e) => void saveTrackPan(track.id, Number(e.currentTarget.value))}
                        />
                      </div>

                      {/* Matrix M / S / L Buttons */}
                      <div className="strip-matrix-buttons">
                        <button
                          type="button"
                          className={`strip-btn btn-mute ${track.muted ? "active" : ""}`}
                          title={track.muted ? "Unmute track" : "Mute track"}
                          onClick={() => void toggleTrackMute(track)}
                        >
                          M
                        </button>
                        <button
                          type="button"
                          className={`strip-btn btn-solo ${track.solo ? "active" : ""}`}
                          title={track.solo ? "Solo track" : "Solo track"}
                          onClick={() => void toggleTrackSolo(track)}
                        >
                          S
                        </button>
                        <button
                          type="button"
                          className={`strip-btn btn-lock ${track.locked ? "active" : ""}`}
                          title={track.locked ? "Unlock track" : "Lock track"}
                          onClick={() => void toggleTrackLock(track)}
                        >
                          {track.locked ? "🔒" : "🔓"}
                        </button>
                      </div>

                      {/* Meter + Vertical Fader Section */}
                      <div className="strip-meter-fader-row">
                        <div className="strip-vu-meter-column" title={`Peak: ${formatDb(trackVu.peakDb)}`}>
                          <div className="vu-meter-pair">
                            <div className="vu-bar-slot">
                              <div
                                className="vu-bar-fill left"
                                style={{ height: `${Math.min(100, (trackVu.left || 0) * 100)}%` }}
                              />
                            </div>
                            <div className="vu-bar-slot">
                              <div
                                className="vu-bar-fill right"
                                style={{ height: `${Math.min(100, (trackVu.right || 0) * 100)}%` }}
                              />
                            </div>
                            {track.audio_processing?.comp_enabled && (
                              <div className="strip-mini-gr-slot" title={`Gain Reduction: ${(trackGr[track.id] || 0).toFixed(1)} dB`}>
                                <div
                                  className="strip-mini-gr-fill"
                                  style={{ height: `${Math.min(100, Math.max(0, -(trackGr[track.id] || 0) * 4.16))}%` }}
                                />
                              </div>
                            )}
                          </div>
                          <div className="vu-db-readout">{formatDb(trackVu.peakDb)}</div>
                        </div>

                        <div className="strip-fader-column">
                          <div className="fader-scale-ticks">
                            <span>+6</span>
                            <span className="unity">0</span>
                            <span>-6</span>
                            <span>-12</span>
                            <span>-24</span>
                            <span>-48</span>
                            <span>-∞</span>
                          </div>
                          <div className="fader-track-container">
                            <input
                              type="range"
                              className="vertical-fader"
                              min="-60"
                              max="6"
                              step="0.5"
                              value={currentGain}
                              onChange={(e) => {
                                const val = Number(e.target.value);
                                setGainDrafts((prev) => ({ ...prev, [track.id]: val }));
                                audioClockMaster.updateTrackGain(track.id, val);
                              }}
                              onPointerUp={(e) => void saveTrackGain(track.id, Number(e.currentTarget.value))}
                              onKeyUp={(e) => void saveTrackGain(track.id, Number(e.currentTarget.value))}
                            />
                          </div>
                        </div>
                      </div>

                      <div className="strip-fader-footer">
                        <button
                          type="button"
                          className="strip-unity-btn"
                          title="Reset track gain to 0.0 dB unity"
                          onClick={() => void saveTrackGain(track.id, 0)}
                        >
                          0 dB
                        </button>
                        <span className="strip-gain-text">
                          {currentGain > 0 ? "+" : ""}{currentGain.toFixed(1)} dB
                        </span>
                      </div>
                    </article>
                  );
                })}
              </div>
            </div>
          </section>
        </div>

        {/* Bottom Tray: Audio Waveforms & Timeline Ruler */}
        <section className="audio-timeline-tray">
          <div className="audio-tray-header">
            <div className="audio-tray-title">
              <strong>AUDIO SEQUENCE TIMELINE &amp; WAVEFORMS</strong>
              <code>{formatTime(playhead)}</code>
            </div>
            <div className="audio-tray-tools">
              <button className="timeline-icon-button" onClick={() => zoomTimeline(1 / 1.2)} title="Zoom Out Timeline">
                <Icon name="zoomOut" />
              </button>
              <button className="timeline-icon-button" onClick={() => zoomTimeline(1.2)} title="Zoom In Timeline">
                <Icon name="zoomIn" />
              </button>
            </div>
          </div>
          <div className="audio-tray-scroller">
            <div className="timeline-content" style={{ width: `${timelineWidth}px`, minWidth: `${timelineWidth}px` }}>
              <div className="timeline-ruler">
                <span className="track-spacer">Time</span>
                <div
                  className="ruler-lane"
                  onPointerDown={beginTimelineScrub}
                  onPointerMove={moveTimelineScrub}
                  onPointerUp={finishTimelineScrub}
                  onPointerCancel={finishTimelineScrub}
                >
                  {Array.from({ length: Math.ceil(timelineWidth / (timelineZoom * 5)) + 1 }, (_, index) => (
                    <span key={index} style={{ left: `${index * timelineZoom * 5}px` }}>
                      {formatTime(index * TICKS_PER_SECOND * 5)}
                    </span>
                  ))}
                  <div
                    className="playhead-grip ruler-playhead-grip"
                    style={{ left: `${(playhead / TICKS_PER_SECOND) * timelineZoom}px` }}
                    role="slider"
                    aria-label="Audio timeline playhead"
                  >
                    <i className="playhead" />
                  </div>
                </div>
              </div>
              {tracks.map((track) => (
                <div
                  className={`track-row ${track.locked ? "is-locked" : ""} ${track.muted ? "is-muted" : ""}`}
                  key={`audio-tray-track-${track.id}`}
                >
                  <div className="track-label">
                    <div className="track-label-top">
                      <b>{track.name}</b>
                      <span>{track.kind}</span>
                    </div>
                    <div className="track-label-btns">
                      <button
                        type="button"
                        className={`track-mini-btn mute-btn ${track.muted ? "active" : ""}`}
                        onClick={() => void toggleTrackMute(track)}
                      >
                        M
                      </button>
                      <button
                        type="button"
                        className={`track-mini-btn solo-btn ${track.solo ? "active" : ""}`}
                        onClick={() => void toggleTrackSolo(track)}
                      >
                        S
                      </button>
                    </div>
                  </div>
                  <div className="track-lane">
                    {track.clips.map((clip, index) => {
                      const item = media.find((entry) => entry.id === clip.media_id);
                      const selected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
                      return (
                        <div
                          className={`timeline-clip ${selected ? "selected" : ""} ${track.kind}`}
                          key={`audio-tray-clip-${track.id}-${clip.media_id}-${index}`}
                          style={{
                            left: `${(clip.timeline_start / TICKS_PER_SECOND) * timelineZoom}px`,
                            width: `${Math.max(34, (clip.duration / TICKS_PER_SECOND) * timelineZoom)}px`,
                          }}
                          onClick={() => {
                            setSelectedClip({ track_id: track.id, clip_index: index });
                            setSelectedClips([{ track_id: track.id, clip_index: index }]);
                          }}
                        >
                          {item?.waveform && item.waveform.length > 0 && (
                            <div className="clip-waveform-container" aria-hidden="true">
                              <svg className="clip-waveform-svg" viewBox="0 0 100 32" preserveAspectRatio="none">
                                <path
                                  d={item.waveform.map((peak, i) => {
                                    const norm = Math.max(1, (peak / 255) * 14);
                                    const x = (i / item.waveform!.length) * 100;
                                    return `M ${x.toFixed(1)} ${(16 - norm).toFixed(1)} L ${x.toFixed(1)} ${(16 + norm).toFixed(1)}`;
                                  }).join(" ")}
                                  stroke="currentColor"
                                  strokeWidth="1.2"
                                  strokeLinecap="round"
                                />
                              </svg>
                            </div>
                          )}
                          <span className="clip-kind-icon">{track.kind === "audio" ? "♪" : "▶"}</span>
                          <strong>{item?.name ?? "Clip"}</strong>
                        </div>
                      );
                    })}
                    <div
                      className="playhead-grip"
                      style={{ left: `${(playhead / TICKS_PER_SECOND) * timelineZoom}px` }}
                    >
                      <i className="playhead" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>
      </main>
    );
  }

  function renderColorWorkspace() {
    const allClips = (project?.sequence.tracks ?? []).flatMap((track) =>
      track.clips.map((clip, index) => ({ track, clip, index, mediaItem: media.find((m) => m.id === clip.media_id) }))
    );
    const updateColorField = (field: keyof ColorAdjustments, value: number) => {
      const next = { ...colorDraft, [field]: value };
      setColorDraft(next);
      void saveColor(next);
    };
    const resetColorGrading = () => {
      const reset = { exposure: 0, contrast: 1.0, saturation: 1.0, hue: 0.0 };
      setColorDraft(reset);
      setColorWheelsDraft(DEFAULT_COLOR_WHEELS);
      void saveColor(reset);
    };

    return (
      <main className="workspace-page color-workspace-page">
        <header className="workspace-sub-header color-workspace-header">
          <div className="workspace-header-title">
            <span className="workspace-eyebrow">COLOR · PRIMARY GRADING &amp; SCOPES</span>
            <h1>Color Grading Studio</h1>
            <p>Real-time GPU video scopes (Waveform, Vectorscope, Parade, Histogram), 3-way color balance wheels &amp; cinematic presets.</p>
          </div>
          <div className="color-header-actions">
            <span className="color-target-info">
              {selectedTimelineClip ? (
                <>Target: <strong>{media.find((m) => m.id === selectedTimelineClip.media_id)?.name ?? "Selected Clip"}</strong></>
              ) : (
                <span className="color-hint-text">Select a clip in the reel below to grade</span>
              )}
            </span>
            <button
              className="btn-secondary"
              onClick={resetColorGrading}
              disabled={!selectedClip}
              title="Reset exposure, contrast, saturation, and color wheels to neutral"
            >
              Reset All
            </button>
            <button
              className="btn-primary-subtle"
              onClick={() => selectDavinciWorkspace("edit")}
            >
              ← Back to Edit
            </button>
          </div>
        </header>

        <div className="color-studio-body">
          {/* Top Half: Side-by-Side Program Monitor + Video Scopes */}
          <div className="color-top-row">
            <section className="color-monitor-stage">
              <div className="color-viewer-container">
                <div className="color-viewer-header">
                  <span className="viewer-title">PROGRAM MONITOR</span>
                  <code>{formatTime(playhead)}</code>
                </div>
                <div className="color-monitor-wrapper">
                  {renderProgramMonitor()}
                </div>
                <div className="color-transport-bar">
                  <button aria-label="Previous frame" onClick={() => stepProgramFrame(-1)}>◀</button>
                  <button
                    className={programPlaying ? "active-toggle" : ""}
                    aria-label={programPlaying ? "Pause playback" : "Play clip"}
                    onClick={() => toggleProgramPlayback()}
                  >
                    {programPlaying ? "⏸ Pause" : "▶ Play"}
                  </button>
                  <button aria-label="Next frame" onClick={() => stepProgramFrame(1)}>▶|</button>
                  <code>{formatTime(playhead)}</code>
                </div>
              </div>
            </section>

            {/* Video Scopes Stage: Real-Time Waveform / Vectorscope / Parade / Histogram */}
            <section className="color-scopes-stage">
              <VideoScopes
                getActiveVideo={() => {
                  if (deckSnapshot?.deckB.visible && deckBRef.current) return deckBRef.current;
                  return deckARef.current;
                }}
                activeFilter={
                  deckSnapshot?.deckB.visible
                    ? (deckSnapshot.deckB.style.filter as string | undefined)
                    : (deckSnapshot?.deckA.style.filter as string | undefined)
                }
              />
            </section>
          </div>

          {/* Bottom Half: 4 Primary Color Wheels Dock + Sliders & Presets */}
          <div className="color-bottom-row">
            <div className="color-wheels-dock-wrapper">
              <ColorWheels
                value={colorWheelsDraft}
                onChange={updateColorWheels}
                disabled={!selectedClip}
              />
            </div>

            <aside className="color-sidebar-dock">
              <div className="color-grading-box">
                <div className="color-section-header">
                  <h3>Primary Sliders</h3>
                  <small>Exposure / Contrast / Sat</small>
                </div>

                <div className="color-sliders-group">
                  <label className="color-slider-item">
                    <div className="slider-label-row">
                      <span>Exposure</span>
                      <b>{colorDraft.exposure > 0 ? `+${colorDraft.exposure.toFixed(1)}` : colorDraft.exposure.toFixed(1)}</b>
                    </div>
                    <input
                      type="range"
                      min="-5"
                      max="5"
                      step="0.1"
                      disabled={!selectedClip}
                      value={colorDraft.exposure}
                      onChange={(e) => setColorDraft((v) => ({ ...v, exposure: Number(e.target.value) }))}
                      onPointerUp={(e) => updateColorField("exposure", Number(e.currentTarget.value))}
                      onKeyUp={(e) => updateColorField("exposure", Number(e.currentTarget.value))}
                    />
                  </label>

                  <label className="color-slider-item">
                    <div className="slider-label-row">
                      <span>Contrast</span>
                      <b>{colorDraft.contrast.toFixed(2)}</b>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="2"
                      step="0.01"
                      disabled={!selectedClip}
                      value={colorDraft.contrast}
                      onChange={(e) => setColorDraft((v) => ({ ...v, contrast: Number(e.target.value) }))}
                      onPointerUp={(e) => updateColorField("contrast", Number(e.currentTarget.value))}
                      onKeyUp={(e) => updateColorField("contrast", Number(e.currentTarget.value))}
                    />
                  </label>

                  <label className="color-slider-item">
                    <div className="slider-label-row">
                      <span>Saturation</span>
                      <b>{colorDraft.saturation.toFixed(2)}</b>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="2"
                      step="0.01"
                      disabled={!selectedClip}
                      value={colorDraft.saturation}
                      onChange={(e) => setColorDraft((v) => ({ ...v, saturation: Number(e.target.value) }))}
                      onPointerUp={(e) => updateColorField("saturation", Number(e.currentTarget.value))}
                      onKeyUp={(e) => updateColorField("saturation", Number(e.currentTarget.value))}
                    />
                  </label>

                  <label className="color-slider-item">
                    <div className="slider-label-row">
                      <span>Hue Tint</span>
                      <b>{((colorDraft.hue ?? 0) * 180).toFixed(0)}°</b>
                    </div>
                    <input
                      type="range"
                      min="-1"
                      max="1"
                      step="0.01"
                      disabled={!selectedClip}
                      value={colorDraft.hue ?? 0}
                      onChange={(e) => setColorDraft((v) => ({ ...v, hue: Number(e.target.value) }))}
                      onPointerUp={(e) => updateColorField("hue", Number(e.currentTarget.value))}
                      onKeyUp={(e) => updateColorField("hue", Number(e.currentTarget.value))}
                    />
                  </label>
                </div>
              </div>

              <div className="color-presets-box">
                <div className="color-section-header">
                  <h3>Look Presets</h3>
                  <small>1-Click</small>
                </div>
                <div className="color-presets-grid compact">
                  {COLOR_PRESETS.map((preset) => (
                    <button
                      key={preset.id}
                      className="color-preset-card compact"
                      disabled={!selectedClip}
                      onClick={() => void applyColorPreset(preset)}
                      title={`Apply ${preset.label} grade`}
                    >
                      <span className="preset-card-icon">{preset.icon}</span>
                      <strong className="preset-card-title">{preset.label}</strong>
                    </button>
                  ))}
                </div>
              </div>
            </aside>
          </div>
        </div>

        {/* Bottom Clips Reel Tray */}
        <section className="clips-reel-tray">
          <div className="reel-tray-header">
            <strong>SEQUENCE CLIPS REEL</strong>
            <small>{allClips.length} clips on timeline · Click any clip to grade</small>
          </div>
          <div className="reel-clips-strip">
            {allClips.map(({ track, clip, index, mediaItem }) => {
              const isSelected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
              return (
                <button
                  key={`reel-${track.id}-${clip.media_id}-${index}`}
                  className={`reel-clip-card ${isSelected ? "selected" : ""}`}
                  onClick={() => {
                    setSelectedClip({ track_id: track.id, clip_index: index });
                    setSelectedClips([{ track_id: track.id, clip_index: index }]);
                    seekProgramTo(clip.timeline_start);
                  }}
                >
                  <div className="reel-clip-badge">
                    <span className={`reel-track-kind ${track.kind}`}>{track.name}</span>
                    <code>{formatTime(clip.timeline_start)}</code>
                  </div>
                  <strong className="reel-clip-name">{mediaItem?.name ?? "Clip"}</strong>
                  <span className="reel-clip-dur">{formatTime(clip.duration)}</span>
                </button>
              );
            })}
          </div>
        </section>
      </main>
    );
  }

  function renderEffectsWorkspace() {
    const allClips = (project?.sequence.tracks ?? []).flatMap((track) =>
      track.clips.map((clip, index) => ({ track, clip, index, mediaItem: media.find((m) => m.id === clip.media_id) }))
    );
    const effects: EffectInstance[] = selectedTimelineClip?.effects ?? [];

    return (
      <main className="workspace-page effects-workspace-page">
        <header className="workspace-sub-header effects-workspace-header">
          <div className="workspace-header-title">
            <span className="workspace-eyebrow">EFFECTS · VISUAL FX &amp; COMPOSITING</span>
            <h1>Visual Effects Lab</h1>
            <p>Compound visual effects stack, blur, crop, vignette, chroma keying, and motion transforms.</p>
          </div>
          <div className="effects-header-actions">
            {selectedClip && (
              <div className="add-effect-control">
                <select
                  className="add-effect-select"
                  defaultValue=""
                  onChange={(e) => {
                    if (e.target.value) {
                      void addEffect(e.target.value as EffectKind);
                      e.target.value = "";
                    }
                  }}
                >
                  <option value="" disabled>＋ Add Effect to Clip…</option>
                  <option value="gaussian_blur">💧 Gaussian Blur</option>
                  <option value="crop">✂️ Crop</option>
                  <option value="vignette">🔘 Vignette</option>
                  <option value="chroma_key">🟩 Chroma Key</option>
                  <option value="color_correction">🎨 Color Correction</option>
                </select>
              </div>
            )}
            <button
              className="btn-primary-subtle"
              onClick={() => selectDavinciWorkspace("edit")}
            >
              ← Back to Edit
            </button>
          </div>
        </header>

        <div className="effects-studio-body">
          {/* Main Stage: Large Program Monitor Viewer + Transport Controls */}
          <section className="effects-monitor-stage">
            <div className="effects-viewer-container">
              <div className="effects-viewer-header">
                <span className="viewer-title">PROGRAM MONITOR &amp; FX COMPOSITE</span>
                <code>{formatTime(playhead)}</code>
              </div>
              <div className="effects-monitor-wrapper">
                {renderProgramMonitor()}
              </div>
              <div className="effects-transport-bar">
                <button aria-label="Previous frame" onClick={() => stepProgramFrame(-1)}>◀</button>
                <button
                  className={programPlaying ? "active-toggle" : ""}
                  aria-label={programPlaying ? "Pause playback" : "Play clip"}
                  onClick={() => toggleProgramPlayback()}
                >
                  {programPlaying ? "⏸ Pause" : "▶ Play"}
                </button>
                <button aria-label="Next frame" onClick={() => stepProgramFrame(1)}>▶|</button>
                <code>{formatTime(playhead)}</code>
              </div>
            </div>
          </section>

          {/* Right Sidebar: Effects Stack */}
          <aside className="effects-stack-sidebar">
            <div className="effects-stack-box">
              <div className="effects-section-header">
                <h3>Active Effect Stack</h3>
                <small>{effects.length} effect{effects.length !== 1 ? "s" : ""} on clip</small>
              </div>

              {!selectedTimelineClip ? (
                <div className="effect-empty-card">
                  <p>Select a clip in the sequence reel below to inspect and configure its effects.</p>
                </div>
              ) : effects.length === 0 ? (
                <div className="effect-empty-card">
                  <p>No effects applied to this clip yet.</p>
                  <small>Choose an effect from the "＋ Add Effect" dropdown above to begin compounding visual effects.</small>
                </div>
              ) : (
                <div className="effects-stack-list">
                  {effects.map((eff, idx) => (
                    <article key={eff.id} className={`effect-stack-card ${eff.enabled ? "is-active" : "is-bypassed"}`}>
                      <header className="effect-card-header">
                        <button
                          type="button"
                          className={`effect-toggle-btn ${eff.enabled ? "enabled" : "bypassed"}`}
                          title={eff.enabled ? "Bypass Effect" : "Enable Effect"}
                          onClick={() => void toggleEffect(eff.id, !eff.enabled)}
                        >
                          {eff.enabled ? "👁" : "👁‍🗨"}
                        </button>
                        <div className="effect-card-title">
                          <strong>{eff.name}</strong>
                          <span className="effect-kind-badge">{eff.kind.replace("_", " ")}</span>
                        </div>
                        <div className="effect-card-actions">
                          <button
                            type="button"
                            className="effect-action-btn"
                            disabled={idx === 0}
                            title="Move Up"
                            onClick={() => void reorderEffect(idx, idx - 1)}
                          >
                            ▲
                          </button>
                          <button
                            type="button"
                            className="effect-action-btn"
                            disabled={idx === effects.length - 1}
                            title="Move Down"
                            onClick={() => void reorderEffect(idx, idx + 1)}
                          >
                            ▼
                          </button>
                          <button
                            type="button"
                            className="effect-action-btn delete"
                            title="Remove Effect"
                            onClick={() => void removeEffect(eff.id)}
                          >
                            ×
                          </button>
                        </div>
                      </header>

                      {eff.enabled && (
                        <div className="effect-card-body">
                          {eff.kind === "color_correction" && (() => {
                            const p = eff.params as ColorCorrectionParams;
                            return (
                              <>
                                <label>
                                  <span>Exposure ({p.exposure > 0 ? `+${p.exposure.toFixed(1)}` : p.exposure.toFixed(1)})</span>
                                  <input
                                    type="range"
                                    min="-5"
                                    max="5"
                                    step="0.1"
                                    value={p.exposure ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, exposure: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Contrast ({p.contrast.toFixed(2)})</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="2"
                                    step="0.01"
                                    value={p.contrast ?? 1}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, contrast: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Saturation ({p.saturation.toFixed(2)})</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="2"
                                    step="0.01"
                                    value={p.saturation ?? 1}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, saturation: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Hue Shift ({Math.round((p.hue ?? 0) * 180)}°)</span>
                                  <input
                                    type="range"
                                    min="-1"
                                    max="1"
                                    step="0.01"
                                    value={p.hue ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "color_correction",
                                      params: { ...p, hue: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </>
                            );
                          })()}

                          {eff.kind === "gaussian_blur" && (() => {
                            const p = eff.params as GaussianBlurParams;
                            return (
                              <label>
                                <span>Blur Radius ({p.sigma.toFixed(1)} px)</span>
                                <input
                                  type="range"
                                  min="0"
                                  max="20"
                                  step="0.1"
                                  value={p.sigma ?? 2}
                                  onChange={(e) => void updateEffectPayload(eff.id, {
                                    kind: "gaussian_blur",
                                    params: { sigma: Number(e.target.value) },
                                  })}
                                />
                              </label>
                            );
                          })()}

                          {eff.kind === "crop" && (() => {
                            const p = eff.params as CropParams;
                            return (
                              <div className="crop-controls-grid">
                                <label>
                                  <span>Top ({p.top ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.top ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, top: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Bottom ({p.bottom ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.bottom ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, bottom: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Left ({p.left ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.left ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, left: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Right ({p.right ?? 0}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="0.5"
                                    value={p.right ?? 0}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "crop",
                                      params: { ...p, right: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </div>
                            );
                          })()}

                          {eff.kind === "vignette" && (() => {
                            const p = eff.params as VignetteParams;
                            return (
                              <>
                                <label>
                                  <span>Intensity ({Math.round((p.intensity ?? 0.5) * 100)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={p.intensity ?? 0.5}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "vignette",
                                      params: { ...p, intensity: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Radius ({p.radius?.toFixed(2) ?? "0.80"})</span>
                                  <input
                                    type="range"
                                    min="0.1"
                                    max="2"
                                    step="0.01"
                                    value={p.radius ?? 0.8}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "vignette",
                                      params: { ...p, radius: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Softness ({Math.round((p.softness ?? 0.5) * 100)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={p.softness ?? 0.5}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "vignette",
                                      params: { ...p, softness: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </>
                            );
                          })()}

                          {eff.kind === "chroma_key" && (() => {
                            const p = eff.params as ChromaKeyParams;
                            return (
                              <>
                                <div className="chroma-color-picker-row">
                                  <span>Key Color</span>
                                  <div className="chroma-inputs">
                                    <input
                                      type="color"
                                      value={p.color ?? "#00ff00"}
                                      onChange={(e) => void updateEffectPayload(eff.id, {
                                        kind: "chroma_key",
                                        params: { ...p, color: e.target.value },
                                      })}
                                    />
                                    <code>{p.color ?? "#00ff00"}</code>
                                  </div>
                                </div>
                                <label>
                                  <span>Tolerance ({Math.round(p.tolerance ?? 25)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="1"
                                    value={p.tolerance ?? 25}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, tolerance: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Edge Softness ({Math.round(p.softness ?? 10)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="50"
                                    step="1"
                                    value={p.softness ?? 10}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, softness: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                                <label>
                                  <span>Spill Suppression ({Math.round((p.spill_suppression ?? 0.5) * 100)}%)</span>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={p.spill_suppression ?? 0.5}
                                    onChange={(e) => void updateEffectPayload(eff.id, {
                                      kind: "chroma_key",
                                      params: { ...p, spill_suppression: Number(e.target.value) },
                                    })}
                                  />
                                </label>
                              </>
                            );
                          })()}
                        </div>
                      )}
                    </article>
                  ))}
                </div>
              )}
            </div>
          </aside>
        </div>

        {/* Bottom Clips Reel Tray */}
        <section className="clips-reel-tray">
          <div className="reel-tray-header">
            <strong>SEQUENCE CLIPS REEL</strong>
            <small>{allClips.length} clips on timeline · Click any clip to apply or edit effects</small>
          </div>
          <div className="reel-clips-strip">
            {allClips.map(({ track, clip, index, mediaItem }) => {
              const isSelected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
              const clipFxCount = clip.effects?.length ?? 0;
              return (
                <button
                  key={`reel-fx-${track.id}-${clip.media_id}-${index}`}
                  className={`reel-clip-card ${isSelected ? "selected" : ""}`}
                  onClick={() => {
                    setSelectedClip({ track_id: track.id, clip_index: index });
                    setSelectedClips([{ track_id: track.id, clip_index: index }]);
                    seekProgramTo(clip.timeline_start);
                  }}
                >
                  <div className="reel-clip-badge">
                    <span className={`reel-track-kind ${track.kind}`}>{track.name}</span>
                    {clipFxCount > 0 && <span className="reel-fx-count-pill">{clipFxCount} FX</span>}
                  </div>
                  <strong className="reel-clip-name">{mediaItem?.name ?? "Clip"}</strong>
                  <span className="reel-clip-dur">{formatTime(clip.duration)}</span>
                </button>
              );
            })}
          </div>
        </section>
      </main>
    );
  }

  function renderImportWorkspace() {
    return <main className="workspace-page import-workspace">
      <header className="workspace-page-heading"><div><span className="workspace-eyebrow">MEDIA · STEP 1</span><h1>Import</h1><p>Bring footage, audio, and graphics into this project.</p></div><button className="primary workspace-heading-action" onClick={() => void importMedia()} disabled={busy}><Icon name="import" /> Browse files…</button></header>
      <section className="import-drop-panel"><div className="import-drop-icon"><Icon name="media" /></div><div><h2>Add media to your project</h2><p>Choose video, audio, or still images from your computer. Imported files stay in their original location.</p></div><button className="primary" onClick={() => void importMedia()} disabled={busy}>＋ Import media</button></section>
      <section className="import-library-section"><header><div><h2>Project media</h2><p>{media.length} item{media.length === 1 ? "" : "s"} in {project?.name ?? "this project"}</p></div><input className="search" type="search" placeholder="Search project media" aria-label="Search project media" value={search} onChange={(event) => setSearch(event.target.value)} /></header>
        {visibleMedia.length ? <div className="import-media-grid">{visibleMedia.map((item) => <button className={`import-media-card ${selectedMediaId === item.id ? "selected" : ""}`} key={item.id} onClick={() => { setSelectedMediaId(item.id); setSelectedClip(null); setSelectedClips([]); selectDavinciWorkspace("edit"); }} title="Select this media and open the Edit workspace"><span className="import-media-icon"><Icon name="media" /></span><strong>{item.name}</strong><small>{item.kind.toUpperCase()} {item.width && item.height ? `· ${item.width} × ${item.height}` : ""}</small><small>{item.duration ? formatTime(item.duration) : missingMediaIds.has(item.id) ? "Missing · relink required" : "Ready to edit"}</small></button>)}</div> : <div className="import-empty"><Icon name="media" /><strong>{search ? "No matching media" : "Your project is ready for media"}</strong><span>{search ? "Try another search." : "Import a clip to begin building your timeline."}</span><button onClick={() => void importMedia()} disabled={busy}>Browse files…</button></div>}
      </section>
    </main>;
  }

  function renderExportWorkspace() {
    const outputResolution = exportResolution === "source"
      ? { width: project?.settings.width ?? 1920, height: project?.settings.height ?? 1080 }
      : resolutionPresets.find((item) => item.value === exportResolution) ?? { width: project?.settings.width ?? 1920, height: project?.settings.height ?? 1080 };
    const outputFrameRate = exportFrameRate === "source"
      ? project?.settings.frame_rate ?? { numerator: 30, denominator: 1 }
      : (() => { const [numerator, denominator] = exportFrameRate.split("/").map(Number); return { numerator, denominator }; })();
    const sequenceEnd = Math.max(0, ...((project?.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.timeline_start + clip.duration))));
    const markStart = project?.sequence.mark_in ?? 0;
    const markEnd = project?.sequence.mark_out ?? sequenceEnd;
    const rangeDuration = exportMarkedRange ? Math.max(0, markEnd - markStart) : sequenceEnd;
    const hasClips = Boolean(project?.sequence.tracks.some((track) => track.clips.length > 0));
    const outputFolder = exportOutputPath ? exportOutputPath.slice(0, Math.max(exportOutputPath.lastIndexOf("/"), exportOutputPath.lastIndexOf("\\"))) : "Choose a destination folder";
    return <main className="export-workspace">
      <aside className="export-destinations"><h2>Destinations</h2><button className="export-destination selected"><Icon name="export" /><span><strong>Media File</strong><small>Render a video file</small></span></button><div className="export-queue"><h3>Queue <span>0</span></h3><p>Exports run immediately when you choose Export video.</p></div></aside>
      <section className="export-settings-panel"><header><span className="workspace-eyebrow">DELIVERY</span><h1>Media File</h1><p>Set a format and size that match where you’ll share this edit.</p></header>
        {/* Live Hardware Acceleration Badge */}
        <div className="export-hw-status-card">
          <div className="hw-status-indicator">
            <span className={`hw-dot ${encoderInfo?.hasHardwareEncoder ? "active" : "fallback"}`} />
            <div className="hw-status-details">
              <strong>
                {encoderInfo?.hasHardwareEncoder
                  ? `Hardware Encoder: ${encoderInfo.preferredEncoder.toUpperCase()}`
                  : `CPU Software Encoding Fallback (${encoderInfo?.preferredEncoder || "x264enc"})`}
              </strong>
              <small>{encoderInfo?.description || "Probing GStreamer hardware acceleration..."}</small>
            </div>
          </div>
          <div className="hw-codecs-list">
            {(encoderInfo?.availableEncoders ?? ["x264enc"]).map((enc) => (
              <span
                key={enc}
                className={`hw-codec-tag ${enc === encoderInfo?.preferredEncoder ? "preferred" : ""}`}
                title={`Detected GStreamer codec: ${enc}`}
              >
                {enc}
              </span>
            ))}
          </div>
        </div>

        {/* 1-Click Target Delivery Presets */}
        <div className="export-presets-shelf">
          <div className="export-presets-shelf-header">
            <strong>1-CLICK DELIVERY PRESETS</strong>
            <small>YouTube, TikTok/Reels 9:16, Instagram 1:1, Cinema 4K</small>
          </div>
          <div className="export-presets-cards-grid">
            {exportPresets.map((preset) => {
              const isSelected = selectedExportPreset === preset.id;
              return (
                <button
                  key={preset.id}
                  type="button"
                  className={`export-preset-card ${isSelected ? "selected" : ""}`}
                  onClick={() => {
                    setSelectedExportPreset(preset.id);
                    setExportResolution(preset.resolution);
                    setExportFrameRate(preset.frameRate);
                    setExportFormat(preset.format);
                  }}
                  title={`Configure for ${preset.name}`}
                >
                  <span className="preset-card-icon">{preset.icon}</span>
                  <div className="preset-card-text">
                    <strong>{preset.name}</strong>
                    <small>{preset.subtitle}</small>
                  </div>
                  <span className="preset-badge">{preset.format.toUpperCase()}</span>
                </button>
              );
            })}
          </div>
        </div>

        <div className="export-form-scroll">
          <label className="export-field">File name<input value={exportFileName} disabled={rendering} onChange={(event) => { setExportFileName(event.target.value); setExportOutputPath(""); }} placeholder="Main Edit" /></label>
          <div className="export-field"><span>Location</span><div className="export-location"><small title={outputFolder}>{outputFolder}</small><button onClick={() => void chooseExportDestination()} disabled={!hasClips || rendering}>Choose…</button></div></div>
          <label className="export-field">Preset<select disabled={rendering} value={exportResolution} onChange={(event) => { setExportResolution(event.target.value); setSelectedExportPreset("custom"); }}><option value="source">Match sequence · {project?.settings.width ?? 1920} × {project?.settings.height ?? 1080}</option>{resolutionPresets.filter((item) => item.value !== "custom").map((item) => <option value={item.value} key={item.value}>{item.label}</option>)}</select></label>
          <label className="export-field">Format<select disabled={rendering} value={exportFormat} onChange={(event) => { setExportFormat(event.target.value as "mp4" | "webm"); setExportOutputPath(""); }}><option value="mp4">H.264 · MP4</option><option value="webm">VP8 · WebM</option></select></label>
          <details className="export-settings-group" open><summary>Video</summary><div className="export-fields-grid"><label className="export-field">Frame size<select disabled={rendering} value={exportResolution} onChange={(event) => { setExportResolution(event.target.value); setSelectedExportPreset("custom"); }}><option value="source">Match sequence</option>{resolutionPresets.filter((item) => item.value !== "custom").map((item) => <option value={item.value} key={item.value}>{item.width} × {item.height}</option>)}</select></label><label className="export-field">Frame rate<select disabled={rendering} value={exportFrameRate} onChange={(event) => { setExportFrameRate(event.target.value); setSelectedExportPreset("custom"); }}>{exportFrameRateOptions.map((item) => <option value={item.value} key={item.value}>{item.value === "source" ? `${item.label} · ${project?.settings.frame_rate.numerator ?? 30}/${project?.settings.frame_rate.denominator ?? 1} fps` : item.label}</option>)}</select></label></div>
            <label className={`export-check ${exportFormat !== "mp4" ? "disabled" : ""}`}><input type="checkbox" checked={useHardwareEncoding} disabled={rendering || exportFormat !== "mp4"} onChange={(event) => setUseHardwareEncoding(event.target.checked)} /><span><strong>Use hardware encoder when available</strong><small>{exportFormat === "mp4" ? "Try your GPU first; software encoding is used if it is unavailable." : "Hardware H.264 applies to MP4 exports."}</small></span></label>
          </details>
          <details className="export-settings-group" open><summary>Audio</summary><div className="export-audio-summary"><span>Audio format</span><strong>{exportFormat === "mp4" ? "AAC" : "Vorbis"}</strong><span>Sample rate</span><strong>{project?.settings.audio_sample_rate ?? 48_000} Hz · source mix</strong></div></details>
          <details className="export-settings-group"><summary>More settings</summary><label className="range-export export-check"><input type="checkbox" checked={exportMarkedRange} disabled={project?.sequence.mark_in == null || project?.sequence.mark_out == null || rendering} onChange={(event) => setExportMarkedRange(event.target.checked)} /><span><strong>Export marked In/Out range</strong><small>Use the sequence markers instead of the full timeline.</small></span></label><div className="export-mark-row"><span>In <b>{project?.sequence.mark_in == null ? "Not set" : formatTime(project.sequence.mark_in)}</b></span><span>Out <b>{project?.sequence.mark_out == null ? "Not set" : formatTime(project.sequence.mark_out)}</b></span><button disabled={project?.sequence.mark_in == null && project?.sequence.mark_out == null} onClick={() => void invokeMenuCommand("timeline.clear_marks")}>Clear marks</button></div></details>
        </div>
      </section>
      <aside className="export-preview-panel"><div className="export-preview-heading"><span>Preview</span><span>{exportFormat === "mp4" ? "H.264" : "VP8"}</span></div><div className="export-preview-stage">{renderPreview(activeClip?.media ?? selectedMedia, true)}</div><div className="export-range-select">Range <strong>{exportMarkedRange ? "Marked In / Out" : "Entire sequence"}</strong></div><div className="export-summary"><h2>Summary</h2><dl><dt>Output</dt><dd>{exportOutputPath || `${exportFileName || project?.name || "Main Edit"}.${exportFormat}`}</dd><dt>Video</dt><dd>{outputResolution.width} × {outputResolution.height} · {outputFrameRate.numerator}/{outputFrameRate.denominator} fps</dd><dt>Engine</dt><dd>{encoderInfo?.hasHardwareEncoder && useHardwareEncoding && exportFormat === "mp4" ? `Hardware Accelerated (${encoderInfo.preferredEncoder})` : "CPU Software (x264enc)"}</dd><dt>Container</dt><dd>{exportFormat === "mp4" ? "H.264 (MP4)" : "VP8 (WebM)"}</dd><dt>Audio</dt><dd>{exportFormat === "mp4" ? "AAC" : "Vorbis"} · {project?.settings.audio_sample_rate ?? 48_000} Hz</dd><dt>Range</dt><dd>{formatTime(rangeDuration)}</dd></dl></div>{renderProgress && <div className="render-progress"><progress max={100} value={renderProgress.fractionPercent} /><small>{renderProgress.phase} · {renderProgress.fractionPercent}%</small></div>}<button className="primary export-submit" disabled={!hasClips || rendering} onClick={() => void exportProject()}>{rendering ? "Rendering…" : "Export video"}</button><small className="export-footer-note">Color controls and track gain are included. Animated keyframes are preview-only.</small></aside>
    </main>;
  }

  return (
    appStage !== "editor" ? renderProjectWelcome() :
    <div className="app-shell" tabIndex={-1} onKeyDown={(event) => {
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) return;
      if (event.key === " " || event.code === "Space") {
        event.preventDefault();
        toggleProgramPlayback();
        return;
      }
      if (event.key.toLowerCase() === "j" && !(event.ctrlKey || event.metaKey || event.altKey)) {
        event.preventDefault();
        handleShuttleKey("j");
        return;
      }
      if (event.key.toLowerCase() === "k" && !(event.ctrlKey || event.metaKey || event.altKey)) {
        event.preventDefault();
        handleShuttleKey("k");
        return;
      }
      if (event.key.toLowerCase() === "l" && !(event.ctrlKey || event.metaKey || event.altKey)) {
        event.preventDefault();
        handleShuttleKey("l");
        return;
      }
      if (event.key === "ArrowLeft" && !(event.ctrlKey || event.metaKey || event.altKey)) {
        event.preventDefault();
        stepProgramFrame(-1);
        return;
      }
      if (event.key === "ArrowRight" && !(event.ctrlKey || event.metaKey || event.altKey)) {
        event.preventDefault();
        stepProgramFrame(1);
        return;
      }
      if (event.key === "Home") {
        event.preventDefault();
        seekProgramTo(0);
        return;
      }
      if (event.key === "End") {
        event.preventDefault();
        const curDuration = Math.max(0, ...(project?.sequence.tracks ?? []).flatMap((t) => t.clips.map((c) => c.timeline_start + c.duration)));
        seekProgramTo(curDuration);
        return;
      }
      if (event.key === "Delete" || event.key === "Backspace") {
        if (selectedTransition) {
          event.preventDefault();
          void removeSelectedTransition();
          return;
        }
        if (selectedClip) {
          event.preventDefault();
          void removeSelectedClip();
          return;
        }
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "t") {
        event.preventDefault();
        void addTitleToTimeline();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "d") {
        event.preventDefault();
        void addTransitionToTimeline();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "l") {
        event.preventDefault();
        void unlinkSelectedClips();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "l") {
        event.preventDefault();
        void linkSelectedClips();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.altKey && event.key.toLowerCase() === "t") {
        event.preventDefault();
        toggleTheme();
        return;
      }
      if (event.shiftKey && ["1", "2", "3", "4", "5", "6"].includes(event.key)) {
        event.preventDefault();
        const map: Record<string, DavinciWorkspace> = {
          "1": "media",
          "2": "edit",
          "3": "color",
          "4": "audio",
          "5": "effects",
          "6": "deliver",
        };
        const ws = map[event.key];
        if (ws) selectDavinciWorkspace(ws);
        return;
      }
      if (!(event.ctrlKey || event.metaKey || event.altKey)) {
        const tool = timelineTools.find((item) => item.shortcut.toLowerCase() === event.key.toLowerCase());
        if (tool) { event.preventDefault(); setTimelineTool(tool.id); return; }
        if (event.key.toLowerCase() === "s") { event.preventDefault(); setSnapEnabled((enabled) => !enabled); return; }
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "q") { event.preventDefault(); void getCurrentWindow().close(); return; }
      if ((event.ctrlKey || event.metaKey) && event.key === "+") { event.preventDefault(); zoomTimeline(1.2); return; }
      if ((event.ctrlKey || event.metaKey) && event.key === "-") { event.preventDefault(); zoomTimeline(1 / 1.2); return; }
      if ((event.ctrlKey || event.metaKey) && event.key === "0") { event.preventDefault(); zoomTimeline(72 / timelineZoom); return; }
      const pressed = `${event.ctrlKey || event.metaKey ? "Ctrl+" : ""}${event.shiftKey ? "Shift+" : ""}${event.key.length === 1 ? event.key.toUpperCase() : event.key}`;
      const command = commands.find((item) => item.shortcut?.toLowerCase() === pressed.toLowerCase());
      if (!command) return;
      event.preventDefault();
      void invokeMenuCommand(command.id).catch((error: unknown) => setNotice(String(error)));
    }}>
      <header className="app-header">
        <div className="header-menu-row">
          <div className="brand">
            <img className="brand-mark" src="/saturn-camera.png" alt="Easy Edits Pro camera logo" />
            <div>
              <strong>Easy Edits Pro</strong>
              <small>{project?.name ?? "Loading project…"}{projectState?.is_dirty ? " •" : ""}</small>
            </div>
          </div>

          {/* DaVinci Resolve Inspired Workspace Bar */}
          <nav className="davinci-workspace-bar" aria-label="DaVinci Resolve Workspaces">
            {davinciWorkspaces.map((item) => {
              const isActive = davinciWorkspace === item.id;
              return (
                <button
                  key={item.id}
                  className={`davinci-workspace-btn ${isActive ? "active" : ""}`}
                  onClick={() => selectDavinciWorkspace(item.id)}
                  data-tooltip={`${item.label} Page (${item.shortcut})`}
                  data-tooltip-pos="bottom"
                  aria-current={isActive ? "page" : undefined}
                >
                  <Icon name={item.icon} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>

          <nav className="menu-bar" aria-label="Application menus">
            {["File", "Edit", "Clip", "Sequence", "Markers", "View", "Window", "Help"].map(renderMenu)}
          </nav>

          <div className="header-actions">
            {/* Background Job Queue Status Pill */}
            <div className="job-queue-wrapper" ref={jobDrawerRef}>
              <button
                className={`job-status-pill ${activeJobs.length > 0 ? "has-active" : ""}`}
                onClick={() => setJobDrawerOpen((prev) => !prev)}
                aria-label="Background Task Queue"
                data-tooltip={
                  activeJobs.length > 0
                    ? `${activeJobs.length} active background task${activeJobs.length === 1 ? "" : "s"}`
                    : "Background Tasks (All idle)"
                }
                data-tooltip-pos="bottom"
              >
                {activeJobs.length > 0 ? (
                  <>
                    <span className="job-pill-spinner" />
                    <span className="job-pill-count">{activeJobs.length}</span>
                    <span className="job-pill-label">
                      {activeJobs[0]?.state.status === "running"
                        ? `${Math.round((activeJobs[0].state.progress || 0) * 100)}%`
                        : "Queued"}
                    </span>
                  </>
                ) : (
                  <>
                    <Icon name="check" />
                    <span className="job-pill-idle">Tasks</span>
                  </>
                )}
              </button>

              {jobDrawerOpen && (
                <div className="job-drawer-popover">
                  <header className="job-drawer-header">
                    <div>
                      <strong>Background Tasks</strong>
                      <small>
                        {activeJobs.length} active · {jobs.length} total
                      </small>
                    </div>
                    <div className="job-drawer-actions">
                      {activeJobs.length > 0 && (
                        <button
                          className="job-cancel-all-btn"
                          onClick={() => void cancelAllJobs()}
                          title="Cancel all running tasks"
                        >
                          Cancel All
                        </button>
                      )}
                      <button
                        className="job-clear-btn"
                        onClick={() => void clearJobHistory()}
                        title="Clear completed history"
                      >
                        Clear History
                      </button>
                    </div>
                  </header>

                  <div className="job-drawer-list">
                    {jobs.length === 0 ? (
                      <div className="job-drawer-empty">No background tasks</div>
                    ) : (
                      jobs.map((job) => (
                        <div key={job.id} className={`job-item job-${job.state.status}`}>
                          <div className="job-item-header">
                            <span className="job-item-label" title={job.label}>
                              {job.label}
                            </span>
                            <span className={`job-item-status-badge badge-${job.state.status}`}>
                              {job.state.status}
                            </span>
                            {(job.state.status === "running" || job.state.status === "queued") && (
                              <button
                                className="job-item-cancel"
                                onClick={() => void cancelJob(job.id)}
                                title="Cancel this task"
                              >
                                ✕
                              </button>
                            )}
                          </div>

                          {job.state.status === "running" && (
                            <div className="job-progress-bar-wrapper">
                              <div
                                className="job-progress-bar-fill"
                                style={{ width: `${Math.round((job.state.progress || 0) * 100)}%` }}
                              />
                              <div className="job-progress-text-row">
                                <small>{job.state.phase || "Processing…"}</small>
                                <small>{Math.round((job.state.progress || 0) * 100)}%</small>
                              </div>
                            </div>
                          )}

                          {job.state.status === "failed" && (
                            <div className="job-error-text">{job.state.error}</div>
                          )}
                        </div>
                      ))
                    )}
                  </div>
                </div>
              )}
            </div>

            <button
              className="theme-toggle-btn"
              onClick={toggleTheme}
              aria-label="Toggle Light / Dark mode"
              data-tooltip={`Switch to ${theme === "dark" ? "Light" : "Dark"} Mode (Ctrl+Alt+T)`}
              data-tooltip-pos="bottom"
            >
              <Icon name={theme === "dark" ? "sun" : "moon"} />
            </button>
            <button
              className="icon-button"
              onClick={() => void toggleFullscreen()}
              aria-label={fullscreen ? "Leave fullscreen" : "Enter fullscreen"}
              data-tooltip={fullscreen ? "Leave Fullscreen (F11)" : "Enter Fullscreen (F11)"}
              data-tooltip-pos="bottom"
            >
              <Icon name={fullscreen ? "restore" : "fullscreen"} />
            </button>
            <button
              className="icon-button close"
              onClick={() => void getCurrentWindow().close()}
              aria-label="Close Easy Edits Pro"
              data-tooltip="Close Easy Edits Pro (Ctrl+Q)"
              data-tooltip-pos="bottom"
            >
              <Icon name="close" />
            </button>
          </div>
        </div>

        <div className="app-toolbar">
          <nav className="file-actions" aria-label="Project actions">
            <button
              onClick={() => { void ensureDiscardIsOkay().then((ok) => { if (ok) { setNewProjectName("Untitled project"); setNewProjectOpen(true); } }); }}
              data-tooltip="New Project (Ctrl+N)"
              data-tooltip-pos="bottom"
            >
              New
            </button>
            <button
              onClick={() => void openProject().catch((error: unknown) => setNotice(String(error)))}
              data-tooltip="Open Project (Ctrl+O)"
              data-tooltip-pos="bottom"
            >
              Open
            </button>
            <button
              onClick={() => void saveProject()}
              data-tooltip="Save Project (Ctrl+S)"
              data-tooltip-pos="bottom"
            >
              Save
            </button>
            <button
              onClick={() => void importMedia()}
              disabled={busy}
              data-tooltip="Import Media Files (Ctrl+I)"
              data-tooltip-pos="bottom"
            >
              Import
            </button>
            <span className="action-divider" />
            <button
              className="history-button"
              onClick={() => void perform("edit.undo")}
              disabled={!undoStatus.enabled}
              data-tooltip={`Undo: ${undoStatus.reason ?? "Undo last action"} (Ctrl+Z)`}
              data-tooltip-pos="bottom"
              aria-label="Undo"
            >
              <Icon name="undo" />
            </button>
            <button
              className="history-button"
              onClick={() => void perform("edit.redo")}
              disabled={!redoStatus.enabled}
              data-tooltip={`Redo: ${redoStatus.reason ?? "Redo action"} (Ctrl+Y)`}
              data-tooltip-pos="bottom"
              aria-label="Redo"
            >
              <Icon name="redo" />
            </button>
          </nav>
        </div>
      </header>

      {davinciWorkspace === "media" ? renderImportWorkspace() :
       davinciWorkspace === "deliver" ? renderExportWorkspace() :
       davinciWorkspace === "audio" ? renderAudioWorkspace() :
       davinciWorkspace === "color" ? renderColorWorkspace() :
       davinciWorkspace === "effects" ? renderEffectsWorkspace() :
       <main className="editor">
        {/* Filmora 15 Inspired Left Panel Tabs */}
        <aside className="media-panel">
          <div className="filmora-tabs" role="tablist" aria-label="Filmora Asset Categories">
            {filmoraTabs.map((tab) => (
              <button
                key={tab.id}
                className={`filmora-tab-btn ${filmoraTab === tab.id ? "active" : ""}`}
                onClick={() => setFilmoraTab(tab.id)}
                data-tooltip={tab.tooltip}
                data-tooltip-pos="bottom"
                role="tab"
                aria-selected={filmoraTab === tab.id}
              >
                <Icon name={tab.icon} />
                <span>{tab.label}</span>
              </button>
            ))}
          </div>

          <div className="filmora-panel-content">
            {filmoraTab === "media" && (
              <>
                <div className="panel-heading">
                  <strong>Project media</strong>
                  <button className="primary" onClick={() => void importMedia()} disabled={busy} data-tooltip="Import Media Files" data-tooltip-pos="bottom">
                    ＋ Import
                  </button>
                </div>
                <input
                  className="search"
                  type="search"
                  placeholder="Search project media…"
                  aria-label="Search project media"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                />
                {visibleMedia.length > 0 ? (
                  <div className="media-list">
                    {visibleMedia.map((item) => (
                      <div className="media-row" key={item.id}>
                        <button
                          className={`media-item ${selectedMediaId === item.id ? "selected" : ""} ${draggedMediaId === item.id ? "pointer-dragging" : ""}`}
                          onPointerDown={(event) => beginMediaPointer(event, item.id)}
                          onPointerMove={moveMediaPointer}
                          onPointerUp={(event) => void finishMediaPointer(event)}
                          onPointerCancel={cancelMediaPointer}
                          onClick={() => {
                            if (suppressMediaClick.current) {
                              suppressMediaClick.current = false;
                              return;
                            }
                            setSelectedMediaId(item.id);
                            setSelectedClip(null);
                            setSelectedClips([]);
                          }}
                          data-tooltip={`Click to inspect · ${item.kind.toUpperCase()} (${item.duration ? formatTime(item.duration) : "Media"})`}
                          data-tooltip-pos="right"
                        >
                          <Icon name="media" />
                          <span>
                            <strong>{item.name}</strong>
                            <small>{missingMediaIds.has(item.id) ? "Missing · relink required" : `${item.kind} · ${item.width && item.height ? `${item.width} × ${item.height}` : item.duration ? formatTime(item.duration) : "Ready"}`}</small>
                          </span>
                        </button>
                        {missingMediaIds.has(item.id) && (
                          <button className="relink-button" onClick={() => void relinkMedia(item.id)} data-tooltip={`Locate file on disk for ${item.name}`}>
                            Relink
                          </button>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="empty-media">
                    <span className="media-icon"><Icon name="media" /></span>
                    <strong>{search ? "No matching media" : "No media imported"}</strong>
                    <small>{search ? "Try another search query." : "Import video, audio, or images to begin editing."}</small>
                    <button onClick={() => void importMedia()} disabled={busy}>Browse files…</button>
                  </div>
                )}
              </>
            )}

            {filmoraTab === "titles" && (
              <>
                <div className="panel-heading">
                  <strong>Title Presets</strong>
                  <small>1-Click Add</small>
                </div>
                <div className="filmora-preset-grid">
                  {titlePresets.map((preset) => (
                    <button
                      key={preset.id}
                      className="filmora-preset-card"
                      onClick={() => void addPresetTitle(preset)}
                      data-tooltip={`Add ${preset.name} at playhead (${formatTime(playhead)})`}
                      data-tooltip-pos="right"
                    >
                      <div className="filmora-preset-preview">
                        <span>{preset.preview}</span>
                      </div>
                      <span className="filmora-preset-name">{preset.name}</span>
                      <span className="filmora-preset-desc">{preset.desc}</span>
                    </button>
                  ))}
                </div>
              </>
            )}

            {filmoraTab === "transitions" && (
              <>
                <div className="panel-heading">
                  <strong>Transitions</strong>
                  <small>1-Click Apply</small>
                </div>
                <div className="filmora-preset-grid">
                  {transitionPresets.map((preset) => (
                    <button
                      key={preset.id}
                      className="filmora-preset-card"
                      onClick={() => void addPresetTransition(preset.id)}
                      data-tooltip={`Apply ${preset.name} at cut/playhead`}
                      data-tooltip-pos="right"
                    >
                      <div className="filmora-preset-preview">
                        <span style={{ fontSize: "16px" }}>{preset.iconSymbol}</span>
                      </div>
                      <span className="filmora-preset-name">{preset.name}</span>
                      <span className="filmora-preset-desc">{preset.desc}</span>
                    </button>
                  ))}
                </div>
              </>
            )}

            {filmoraTab === "audio" && (
              <>
                <div className="panel-heading">
                  <strong>Audio Clips</strong>
                </div>
                <div className="media-list">
                  {media.filter((item) => item.kind === "audio").map((item) => (
                    <div className="media-row" key={item.id}>
                      <button
                        className={`media-item ${selectedMediaId === item.id ? "selected" : ""}`}
                        onClick={() => setSelectedMediaId(item.id)}
                        data-tooltip={`Audio track · ${item.duration ? formatTime(item.duration) : "Audio"}`}
                        data-tooltip-pos="right"
                      >
                        <Icon name="audio" />
                        <span>
                          <strong>{item.name}</strong>
                          <small>{item.duration ? formatTime(item.duration) : "Audio clip"}</small>
                        </span>
                      </button>
                    </div>
                  ))}
                  {media.filter((item) => item.kind === "audio").length === 0 && (
                    <div className="empty-media">
                      <span className="media-icon"><Icon name="audio" /></span>
                      <strong>No audio imported</strong>
                      <small>Import background music, voiceover, or sound effects.</small>
                    </div>
                  )}
                </div>
              </>
            )}

            {filmoraTab === "stock" && (
              <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                <div className="panel-heading">
                  <strong>Stock Media</strong>
                  <small>Openverse</small>
                </div>
                <input
                  className="search"
                  value={assetQuery}
                  maxLength={200}
                  placeholder="Search photos, audio…"
                  onChange={(event) => setAssetQuery(event.target.value)}
                />
                <div style={{ display: "flex", gap: "6px", margin: "4px 9px" }}>
                  <select
                    style={{ flex: 1, padding: "5px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
                    value={assetType}
                    onChange={(event) => setAssetType(event.target.value as typeof assetType)}
                  >
                    <option value="images">Images</option>
                    <option value="audio">Audio</option>
                  </select>
                  <LoadingButton
                    className="primary"
                    disabled={busy || !assetQuery.trim()}
                    onAction={searchAssets}
                    pendingLabel="…"
                    successLabel="Done"
                    errorLabel="Failed"
                  >
                    Search
                  </LoadingButton>
                </div>
                <div className="asset-results" style={{ padding: "0 9px" }}>
                  {assetResults.map((asset) => (
                    <article className="asset-result" key={`${asset.id}-${asset.mediaUrl}`}>
                      {asset.thumbnailUrl && <img src={asset.thumbnailUrl} alt="" loading="lazy" />}
                      <strong>{asset.title}</strong>
                      <small>{asset.creator ? `By ${asset.creator}` : "Openverse"} · {asset.license ?? "FOSS"}</small>
                      <div className="asset-actions">
                        <button disabled={busy} onClick={() => void addLibraryAsset(asset)}>Add to Bin</button>
                      </div>
                    </article>
                  ))}
                </div>
              </div>
            )}

            {filmoraTab === "ai" && (
              <div style={{ padding: "4px 9px", display: "flex", flexDirection: "column", gap: "8px" }}>
                <div className="panel-heading" style={{ padding: 0 }}>
                  <strong>Local AI Assistant</strong>
                  <small>Ollama</small>
                </div>
                <label style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                  Model
                  <input
                    style={{ width: "100%", marginTop: "3px", padding: "5px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px" }}
                    value={aiModel}
                    maxLength={100}
                    onChange={(event) => setAiModel(event.target.value)}
                  />
                </label>
                <label style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                  Prompt
                  <textarea
                    rows={4}
                    style={{ width: "100%", marginTop: "3px", padding: "5px", background: "var(--bg-input)", color: "var(--text-main)", border: "1px solid var(--border-color)", borderRadius: "4px", resize: "vertical" }}
                    value={aiPrompt}
                    placeholder="Ask for editing ideas, shot pacing, or script notes…"
                    onChange={(event) => setAiPrompt(event.target.value)}
                  />
                </label>
                <button className="primary" disabled={busy || !aiPrompt.trim() || !aiModel.trim()} onClick={() => void askAi()}>
                  Generate Suggestions
                </button>
                {aiReply && <pre className="ai-reply">{aiReply}</pre>}
              </div>
            )}
          </div>
        </aside>

        <section className="work-area">
          <div className="monitors">
            <section className="monitor">
              <div className="panel-tabs"><span className="selected">Source Monitor</span></div>
              <div className="monitor-name">{selectedMedia?.name ?? "No source selected"}</div>
              {renderPreview(selectedMedia)}
            </section>
            <section className="monitor">
              <div className="panel-tabs"><span className="selected">Program Timeline</span></div>
              <div className="monitor-name">{project?.sequence.name ?? "Sequence 1"}</div>
              {renderProgramMonitor()}
              <div className="transport">
                <button aria-label="Previous frame" data-tooltip="Previous Frame (Left Arrow)" data-tooltip-pos="top" onClick={() => stepProgramFrame(-1)}>◀</button>
                <button className={programPlaying ? "active-toggle" : ""} aria-label={programPlaying ? "Pause playback" : "Play clip"} data-tooltip={programPlaying ? "Pause Playback (Space / K)" : "Play Timeline (Space / L)"} data-tooltip-pos="top" onClick={() => toggleProgramPlayback()}>{programPlaying ? "Ⅱ" : "▶"}</button>
                <button aria-label="Next frame" data-tooltip="Next Frame (Right Arrow)" data-tooltip-pos="top" onClick={() => stepProgramFrame(1)}>▶|</button>
                {shuttleSpeed !== 1 && programPlaying && (
                  <span className="transport-shuttle" title="Shuttle speed">{shuttleSpeed}x</span>
                )}
                <code>{formatTime(playhead)}</code>
              </div>
            </section>
          </div>

          <section className="timeline">
            {/* Adobe Premiere Pro Style Tool Rail */}
            <aside className="timeline-tool-rail" aria-label="Timeline editing tools">
              {timelineTools.map((tool) => (
                <button
                  key={tool.id}
                  className={`timeline-tool-button ${timelineTool === tool.id ? "active" : ""}`}
                  onClick={() => setTimelineTool(tool.id)}
                  aria-label={tool.label}
                  aria-pressed={timelineTool === tool.id}
                  data-tooltip={`${tool.label} (${tool.shortcut})`}
                  data-tooltip-pos="right"
                >
                  <Icon name={tool.icon} />
                  <span>{tool.shortLabel}</span>
                </button>
              ))}
            </aside>
            <div className="timeline-workbench">
              <div className="timeline-toolbar">
                <div className="timeline-toolbar-start">
                  <strong>Timeline</strong>
                  <code>{formatTime(playhead)} / {formatTime(Math.max(0, ...((project?.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.timeline_start + clip.duration)))))}</code>
                </div>
                <div className="timeline-toolbar-tools" aria-label="Timeline actions">
                  <button className={`timeline-icon-button ${snapEnabled ? "active-toggle" : ""}`} aria-pressed={snapEnabled} onClick={() => setSnapEnabled((enabled) => !enabled)} data-tooltip={`Snap to Frame: ${snapEnabled ? "On" : "Off"} (S)`} data-tooltip-pos="top" aria-label="Toggle snapping"><Icon name="snap" /></button>
                  <button className="timeline-icon-button" disabled={!selectedMedia || busy} onClick={() => void addSelectedToTimeline()} data-tooltip="Add Selected Media to Timeline (Shift+A)" data-tooltip-pos="top" aria-label="Add selected media"><Icon name="add" /></button>
                  <button className="timeline-icon-button" disabled={busy} onClick={() => void addTitleToTimeline()} data-tooltip="Add Title / Lower-Third Graphic (Ctrl+T)" data-tooltip-pos="top" aria-label="Add Title"><Icon name="title" /></button>
                  <button className="timeline-icon-button" disabled={busy} onClick={() => void addTransitionToTimeline()} data-tooltip="Add Transition at Cut (Ctrl+D)" data-tooltip-pos="top" aria-label="Add Transition"><Icon name="transition" /></button>
                  <span className="toolbar-divider" />
                  <button className="timeline-icon-button" disabled={!undoStatus.enabled || busy} onClick={() => void perform("edit.undo")} data-tooltip={`Undo: ${undoStatus.reason ?? "Undo"} (Ctrl+Z)`} data-tooltip-pos="top" aria-label="Undo"><Icon name="undo" /></button>
                  <button className="timeline-icon-button" disabled={!redoStatus.enabled || busy} onClick={() => void perform("edit.redo")} data-tooltip={`Redo: ${redoStatus.reason ?? "Redo"} (Ctrl+Y)`} data-tooltip-pos="top" aria-label="Redo"><Icon name="redo" /></button>
                  <span className="toolbar-divider" />
                  <button className="timeline-icon-button" disabled={!selectedClip || busy} onClick={() => void splitSelectedClip()} data-tooltip="Split Selected Clip at Playhead (Ctrl+K)" data-tooltip-pos="top" aria-label="Split clip"><Icon name="split" /></button>
                  <button className="timeline-icon-button" disabled={!selectedClip || busy} onClick={() => void rippleDeleteSelectedClip()} data-tooltip="Ripple Delete Selected Clip (Shift+Del)" data-tooltip-pos="top" aria-label="Ripple delete"><Icon name="ripple" /></button>
                  <button className="timeline-icon-button" disabled={!selectedClip || busy} onClick={() => void removeSelectedClip()} data-tooltip="Remove Selected Clip (Del)" data-tooltip-pos="top" aria-label="Remove clip"><Icon name="remove" /></button>
                  <span className="toolbar-divider" />
                  <button className="timeline-icon-button" disabled={selectedClips.length < 2 || busy} onClick={() => void linkSelectedClips()} data-tooltip="Link Selected Clips (Ctrl+L)" data-tooltip-pos="top" aria-label="Link clips"><Icon name="link" /></button>
                  <button className="timeline-icon-button" disabled={(!selectedClip && selectedClips.length === 0) || busy} onClick={() => void unlinkSelectedClips()} data-tooltip="Unlink Selected Clips (Ctrl+Shift+L)" data-tooltip-pos="top" aria-label="Unlink clips"><Icon name="unlink" /></button>
                  <span className="toolbar-divider" />
                  <button className="timeline-icon-button" disabled={busy} onClick={() => void invokeMenuCommand("timeline.set_mark_in")} data-tooltip="Set In Point (I)" data-tooltip-pos="top" aria-label="Set In point"><Icon name="markIn" /></button>
                  <button className="timeline-icon-button" disabled={busy} onClick={() => void invokeMenuCommand("timeline.set_mark_out")} data-tooltip="Set Out Point (O)" data-tooltip-pos="top" aria-label="Set Out point"><Icon name="markOut" /></button>
                  <button className="timeline-icon-button" disabled={project?.sequence.mark_in == null && project?.sequence.mark_out == null} onClick={() => void invokeMenuCommand("timeline.clear_marks")} data-tooltip="Clear In/Out Markers (Alt+X)" data-tooltip-pos="top" aria-label="Clear markers"><Icon name="clearMarks" /></button>
                  <span className="toolbar-divider" />
                  <button className="timeline-icon-button" onClick={() => zoomTimeline(1 / 1.2)} data-tooltip="Zoom Out Timeline (Ctrl+-)" data-tooltip-pos="top" aria-label="Zoom out"><Icon name="zoomOut" /></button>
                  <button className="timeline-icon-button" onClick={() => zoomTimeline(1.2)} data-tooltip="Zoom In Timeline (Ctrl++)" data-tooltip-pos="top" aria-label="Zoom in"><Icon name="zoomIn" /></button>
                </div>
              </div>
              <div ref={timelineScrollerRef} className={`timeline-scroller ${timelineTool === "hand" ? "hand-mode" : ""}`} onPointerDown={handleTimelinePanStart} onPointerMove={handleTimelinePanMove} onPointerUp={handleTimelinePanEnd} onPointerCancel={handleTimelinePanEnd}>
                <div className="timeline-content" style={{ width: `${timelineWidth}px`, minWidth: `${timelineWidth}px` }}>
                  <div className="timeline-ruler"><span className="track-spacer">Time</span><div className="ruler-lane" onPointerDown={beginTimelineScrub} onPointerMove={moveTimelineScrub} onPointerUp={finishTimelineScrub} onPointerCancel={finishTimelineScrub}>{project?.sequence.mark_in != null && <i className="sequence-marker in-point" style={{ left: `${project.sequence.mark_in / TICKS_PER_SECOND * timelineZoom}px` }} title={`In · ${formatTime(project.sequence.mark_in)}`}><Icon name="markIn" /></i>}{project?.sequence.mark_out != null && <i className="sequence-marker out-point" style={{ left: `${project.sequence.mark_out / TICKS_PER_SECOND * timelineZoom}px` }} title={`Out · ${formatTime(project.sequence.mark_out)}`}><Icon name="markOut" /></i>}{Array.from({ length: Math.ceil(timelineWidth / (timelineZoom * 5)) + 1 }, (_, index) => <span key={index} style={{ left: `${index * timelineZoom * 5}px` }}>{formatTime(index * TICKS_PER_SECOND * 5)}</span>)}<div className="playhead-grip ruler-playhead-grip" style={{ left: `${playhead / TICKS_PER_SECOND * timelineZoom}px` }} role="slider" aria-label="Timeline playhead" aria-valuenow={playhead / TICKS_PER_SECOND} aria-valuemin={0} aria-valuemax={timelineEndSeconds} onPointerDown={beginTimelineScrub} onPointerMove={moveTimelineScrub} onPointerUp={finishTimelineScrub} onPointerCancel={finishTimelineScrub}><i className="playhead" /></div></div></div>
                  {(project?.sequence.tracks ?? []).map((track) => <div className={`track-row ${track.locked ? "is-locked" : ""} ${track.muted ? "is-muted" : ""}`} key={track.id}>
                    <div className="track-label">
                      <div className="track-label-top">
                        <b>{track.name}</b>
                        <span>{track.kind}</span>
                      </div>
                      <div className="track-label-btns">
                        <button
                          type="button"
                          className={`track-mini-btn mute-btn ${track.muted ? "active" : ""}`}
                          title={track.muted ? "Unmute track" : "Mute track"}
                          onClick={(e) => { e.stopPropagation(); void toggleTrackMute(track); }}
                        >
                          M
                        </button>
                        <button
                          type="button"
                          className={`track-mini-btn solo-btn ${track.solo ? "active" : ""}`}
                          title={track.solo ? "Unsolo track" : "Solo track"}
                          onClick={(e) => { e.stopPropagation(); void toggleTrackSolo(track); }}
                        >
                          S
                        </button>
                        <button
                          type="button"
                          className={`track-mini-btn lock-btn ${track.locked ? "active" : ""}`}
                          title={track.locked ? "Unlock track" : "Lock track"}
                          onClick={(e) => { e.stopPropagation(); void toggleTrackLock(track); }}
                        >
                          {track.locked ? "🔒" : "🔓"}
                        </button>
                      </div>
                    </div>
                    <div className={`track-lane ${track.locked ? "track-lane-locked" : ""} ${mediaDropTargetTrackId === track.id ? "media-drop-target" : ""}`} data-track-id={track.id} onClick={() => { setSelectedTransition(null); }}>
                      {(track.transitions ?? []).map((trans) => {
                        const cutX = (trans.cut_at / TICKS_PER_SECOND) * timelineZoom;
                        const durPx = Math.max(26, (trans.duration / TICKS_PER_SECOND) * timelineZoom);
                        const left = Math.max(0, cutX - durPx / 2);
                        const isSelected = selectedTransition?.track_id === track.id && selectedTransition?.transition.id === trans.id;
                        const label = trans.kind.replace(/_/g, " ");
                        return (
                          <div
                            key={`trans-${track.id}-${trans.id}`}
                            className={`timeline-transition-badge ${isSelected ? "selected" : ""}`}
                            style={{ left: `${left}px`, width: `${durPx}px` }}
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedTransition({ track_id: track.id, transition: trans });
                              setSelectedClip(null);
                              setSelectedClips([]);
                            }}
                            title={`Transition: ${label} (${(trans.duration / TICKS_PER_SECOND).toFixed(1)}s)`}
                          >
                            <span className="transition-icon"><Icon name="transition" /></span>
                            <span className="transition-name">{label}</span>
                          </div>
                        );
                      })}
                      {track.clips.map((clip, index) => {
                        const item = media.find((entry) => entry.id === clip.media_id);
                        const isTitle = item?.kind === "title";
                        const selected = (selectedClip?.track_id === track.id && selectedClip.clip_index === index) || selectedClips.some((item) => item.track_id === track.id && item.clip_index === index);
                        const isDirectDrag = clipDragOffset?.trackId === track.id && clipDragOffset.clipIndex === index;
                        const isLinkedDrag = clipDragOffset != null && clip.group_id != null && linkedClipsDrag.current.some((l) => l.trackId === track.id && l.clipIndex === index);
                        const activeDrag = isDirectDrag ? clipDragOffset : isLinkedDrag ? { x: clipDragOffset.x, y: 0 } : null;
                        return <div className={`timeline-clip ${selected ? "selected" : ""} ${track.kind} ${isTitle ? "title-clip" : ""} tool-${timelineTool} ${activeDrag ? "dragging" : ""}`} key={`${track.id}-${clip.media_id}-${index}-${clip.timeline_start}`} style={{ left: `${clip.timeline_start / TICKS_PER_SECOND * timelineZoom}px`, width: `${Math.max(34, clip.duration / TICKS_PER_SECOND * timelineZoom)}px`, transform: activeDrag ? `translate3d(${activeDrag.x}px, ${activeDrag.y}px, 0)` : undefined }} onPointerDown={(event) => beginTimelineClipPointer(event, track.id, index, clip)} onPointerMove={moveTimelineClipPointer} onPointerUp={(event) => void finishTimelineClipPointer(event)} onPointerCancel={handleTimelineClipPointerCancel} onClick={(event) => void handleTimelineClipClick(event, track.id, index, clip)} title={`${item?.name ?? "Clip"} · ${timelineTools.find((tool) => tool.id === timelineTool)?.label}`}>
                          {track.kind === "audio" && item?.waveform && item.waveform.length > 0 && (
                            <div className="clip-waveform-container" aria-hidden="true">
                              <svg className="clip-waveform-svg" viewBox="0 0 100 32" preserveAspectRatio="none">
                                <path
                                  d={item.waveform.map((peak, i) => {
                                    const norm = Math.max(1, (peak / 255) * 14);
                                    const x = (i / item.waveform!.length) * 100;
                                    return `M ${x.toFixed(1)} ${(16 - norm).toFixed(1)} L ${x.toFixed(1)} ${(16 + norm).toFixed(1)}`;
                                  }).join(" ")}
                                  stroke="currentColor"
                                  strokeWidth="1.2"
                                  strokeLinecap="round"
                                />
                              </svg>
                            </div>
                          )}
                          <span className="clip-kind-icon">{isTitle ? "T" : track.kind === "audio" ? "♪" : "▶"}</span>
                          <strong>{item?.name ?? "Missing media"}</strong>
                          {clip.speed && clip.speed !== 1.0 && (
                            <span className="clip-speed-badge">⚡{Math.round(clip.speed * 100)}%</span>
                          )}
                          {clip.group_id != null && (
                            <span className="clip-group-badge" title={`Linked Group #${clip.group_id}`}>🔗</span>
                          )}
                        </div>;
                      })}
                      <div className="playhead-grip" style={{ left: `${playhead / TICKS_PER_SECOND * timelineZoom}px` }} role="slider" aria-label="Timeline playhead" aria-valuenow={playhead / TICKS_PER_SECOND} aria-valuemin={0} aria-valuemax={timelineEndSeconds} onPointerDown={beginTimelineScrub} onPointerMove={moveTimelineScrub} onPointerUp={finishTimelineScrub} onPointerCancel={finishTimelineScrub}><i className="playhead" /></div>
                    </div>
                  </div>)}
                </div>
              </div>
            </div>
          </section>
        </section>

        <aside className="tool-panel" aria-live="polite">{renderInspector()}</aside>
      </main>}
      <footer className="status-bar"><span>{busy ? "Working…" : notice || "Autosave recovery enabled · saves every 30 seconds"}</span><span>{connection}</span></footer>
      {newProjectOpen && <div className="dialog-backdrop" role="presentation"><form className="new-project-dialog" onSubmit={(event) => { event.preventDefault(); void createProject(); }}><h2>New project</h2><label>Project name<input autoFocus value={newProjectName} onChange={(event) => setNewProjectName(event.target.value)} /></label><div className="dialog-actions"><button type="button" onClick={() => setNewProjectOpen(false)}>Cancel</button><button className="primary" type="submit" disabled={!newProjectName.trim()}>Create</button></div></form></div>}
    </div>
  );
}

export default App;
