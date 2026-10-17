import type { CSSProperties } from "react";

export const TICKS_PER_SECOND = 254_016_000_000;

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

export interface Keyframe {
  property: KeyframeProperty;
  time: number;
  value: number;
}

export interface ColorAdjustments {
  exposure: number;
  contrast: number;
  saturation: number;
}

export type EffectKind =
  | "color_correction"
  | "gaussian_blur"
  | "crop"
  | "vignette"
  | "chroma_key";

export interface ColorCorrectionParams {
  exposure: number;
  contrast: number;
  saturation: number;
  hue: number;
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

export type EffectPayload =
  | { kind: "color_correction"; params: ColorCorrectionParams }
  | { kind: "gaussian_blur"; params: GaussianBlurParams }
  | { kind: "crop"; params: CropParams }
  | { kind: "vignette"; params: VignetteParams }
  | { kind: "chroma_key"; params: ChromaKeyParams };

export interface EffectInstance {
  id: number;
  name: string;
  enabled: boolean;
  kind: EffectKind;
  params: ColorCorrectionParams | GaussianBlurParams | CropParams | VignetteParams | ChromaKeyParams;
}

export interface Clip {
  media_id: number;
  timeline_start: number;
  source_in: number;
  duration: number;
  color: ColorAdjustments;
  effects?: EffectInstance[];
  keyframes: Keyframe[];
}

export interface Track {
  id: number;
  kind: "video" | "audio";
  name: string;
  gain_db: number;
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

export interface MediaAsset {
  id: number;
  name: string;
  path: string;
  kind: "video" | "audio" | "image" | "title" | "unknown";
  duration: number | null;
  width: number | null;
  height: number | null;
  title?: TitleProperties | null;
  waveform?: number[] | null;
}

export interface ProjectDocument {
  project: {
    name: string;
    media: MediaAsset[];
    sequence: {
      name: string;
      tracks: Track[];
      mark_in: number | null;
      mark_out: number | null;
    };
    settings: {
      width: number;
      height: number;
      frame_rate: { numerator: number; denominator: number };
      audio_sample_rate: number;
    };
  };
}

/**
 * AudioClockMaster manages a hardware-synchronized Web Audio context
 * providing monotonic, drift-free master clock timing and per-track audio mixing.
 */
class AudioClockMaster {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private trackGains = new Map<number, GainNode>();

  getContext(): AudioContext | null {
    if (typeof window === "undefined") return null;
    if (!this.ctx) {
      const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (AudioCtx) {
        this.ctx = new AudioCtx();
        this.masterGain = this.ctx.createGain();
        this.masterGain.connect(this.ctx.destination);
      }
    }
    if (this.ctx && this.ctx.state === "suspended") {
      void this.ctx.resume().catch(() => {
        // User gesture may be required before resuming audio
      });
    }
    return this.ctx;
  }

  getCurrentTime(): number {
    const ctx = this.getContext();
    return ctx ? ctx.currentTime : performance.now() / 1000;
  }

  getTrackGainNode(trackId: number, gainDb = 0): GainNode | null {
    const ctx = this.getContext();
    if (!ctx || !this.masterGain) return null;
    let node = this.trackGains.get(trackId);
    if (!node) {
      node = ctx.createGain();
      node.connect(this.masterGain);
      this.trackGains.set(trackId, node);
    }
    const linearGain = Math.max(0, Math.min(2, 10 ** (gainDb / 20)));
    node.gain.setValueAtTime(linearGain, ctx.currentTime);
    return node;
  }

  updateTrackGain(trackId: number, gainDb: number): void {
    const ctx = this.getContext();
    const node = this.trackGains.get(trackId);
    if (ctx && node) {
      const linearGain = Math.max(0, Math.min(2, 10 ** (gainDb / 20)));
      node.gain.setTargetAtTime(linearGain, ctx.currentTime, 0.02);
    }
  }

  smoothMicroFade(gainNode: GainNode, fadeIn: boolean, durationSec = 0.015): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const now = ctx.currentTime;
    if (fadeIn) {
      gainNode.gain.setValueAtTime(0.001, now);
      gainNode.gain.exponentialRampToValueAtTime(1.0, now + durationSec);
    } else {
      gainNode.gain.setValueAtTime(1.0, now);
      gainNode.gain.exponentialRampToValueAtTime(0.001, now + durationSec);
    }
  }
}

export const audioClockMaster = new AudioClockMaster();

/**
 * Computes an animated keyframe property value with linear interpolation.
 */
export function interpolateKeyframe(
  clip: Clip | undefined,
  property: KeyframeProperty,
  clipTimeTicks: number,
  fallback: number
): number {
  if (!clip || !clip.keyframes) return fallback;
  const values = clip.keyframes.filter((k) => k.property === property).sort((a, b) => a.time - b.time);
  if (!values.length) return fallback;
  if (clipTimeTicks <= values[0].time) return values[0].value;
  const last = values[values.length - 1];
  if (clipTimeTicks >= last.time) return last.value;
  const nextIndex = values.findIndex((k) => k.time >= clipTimeTicks);
  const prev = values[nextIndex - 1];
  const next = values[nextIndex];
  if (!prev || !next) return fallback;
  const span = next.time - prev.time;
  if (span <= 0) return prev.value;
  const ratio = (clipTimeTicks - prev.time) / span;
  return prev.value + (next.value - prev.value) * ratio;
}

export interface TransitionBlend {
  active: boolean;
  kind: TransitionKind;
  progress: number; // 0.0 to 1.0
  deckAStyle: CSSProperties;
  deckBStyle: CSSProperties;
}

/**
 * Calculates real-time visual transition styling for Dual-Deck video playback.
 */
export function computeTransitionBlend(
  transition: Transition | null,
  playhead: number
): TransitionBlend {
  if (!transition) {
    return {
      active: false,
      kind: "crossfade",
      progress: 0,
      deckAStyle: { opacity: 1 },
      deckBStyle: { opacity: 0, pointerEvents: "none" },
    };
  }

  const halfDur = Math.floor(transition.duration / 2);
  const start = transition.cut_at - halfDur;
  const end = transition.cut_at + halfDur;

  if (playhead < start || playhead > end) {
    return {
      active: false,
      kind: transition.kind,
      progress: playhead < start ? 0 : 1,
      deckAStyle: { opacity: playhead < start ? 1 : 0 },
      deckBStyle: { opacity: playhead >= end ? 1 : 0 },
    };
  }

  const duration = Math.max(1, end - start);
  const rawProgress = (playhead - start) / duration;
  const p = Math.max(0, Math.min(1, rawProgress));

  switch (transition.kind) {
    case "crossfade":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 1 - p },
        deckBStyle: { opacity: p },
      };

    case "dip_to_black":
      if (p < 0.5) {
        return {
          active: true,
          kind: transition.kind,
          progress: p,
          deckAStyle: { opacity: Math.max(0, 1 - p * 2) },
          deckBStyle: { opacity: 0 },
        };
      }
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 0 },
        deckBStyle: { opacity: Math.max(0, (p - 0.5) * 2) },
      };

    case "dip_to_white": {
      const flash = Math.sin(p * Math.PI) * 2.5;
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: {
          opacity: 1 - p,
          filter: `brightness(${1 + flash})`,
        },
        deckBStyle: {
          opacity: p,
          filter: `brightness(${1 + flash})`,
        },
      };
    }

    case "wipe_left":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 1 },
        deckBStyle: {
          opacity: 1,
          clipPath: `inset(0 0 0 ${(1 - p) * 100}%)`,
        },
      };

    case "wipe_right":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 1 },
        deckBStyle: {
          opacity: 1,
          clipPath: `inset(0 ${(1 - p) * 100}% 0 0)`,
        },
      };

    case "slide_left":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: {
          transform: `translateX(${-p * 100}%)`,
        },
        deckBStyle: {
          transform: `translateX(${(1 - p) * 100}%)`,
        },
      };

    case "slide_right":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: {
          transform: `translateX(${p * 100}%)`,
        },
        deckBStyle: {
          transform: `translateX(${-(1 - p) * 100}%)`,
        },
      };

    default:
      return {
        active: true,
        kind: "crossfade",
        progress: p,
        deckAStyle: { opacity: 1 - p },
        deckBStyle: { opacity: p },
      };
  }
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
  hasVideoMedia: boolean;
}

/**
 * Resolves the URL for a media asset, preferring prepared WebM/VP8 proxies for video.
 */
export function resolveMediaStreamUrl(
  mediaAsset: MediaAsset | null | undefined,
  preparedPreviews: Record<number, string>,
  mediaServerPort: number
): string {
  if (!mediaAsset || mediaServerPort <= 0) return "";
  const proxyPath = preparedPreviews[mediaAsset.id];
  const targetPath = proxyPath || mediaAsset.path;
  return `http://127.0.0.1:${mediaServerPort}/media?path=${encodeURIComponent(targetPath)}`;
}

/**
 * Builds the complete dual-deck video and multi-track audio playback evaluation
 * for any given timeline playhead position.
 */
export function evaluateTimelinePlayback(
  project: ProjectDocument | null,
  playhead: number,
  mediaList: MediaAsset[],
  preparedPreviews: Record<number, string>,
  mediaServerPort: number
): TimelinePlaybackSnapshot {
  const emptyResult: TimelinePlaybackSnapshot = {
    playhead,
    sequenceDuration: 0,
    deckA: { mediaId: null, src: "", currentTime: 0, visible: false, style: {}, clip: null, track: null, isPreload: false },
    deckB: { mediaId: null, src: "", currentTime: 0, visible: false, style: {}, clip: null, track: null, isPreload: false },
    activeDeck: "A",
    inTransition: false,
    transitionKind: null,
    activeAudioClips: [],
    activeTitleClip: null,
    hasVideoMedia: false,
  };

  if (!project) return emptyResult;

  const tracks = project.project.sequence.tracks;
  const projectWidth = project.project.settings.width || 1920;
  const projectHeight = project.project.settings.height || 1080;

  // Compute sequence duration across all tracks
  let maxEnd = 0;
  for (const track of tracks) {
    for (const clip of track.clips) {
      const end = clip.timeline_start + clip.duration;
      if (end > maxEnd) maxEnd = end;
    }
  }
  const sequenceDuration = maxEnd;

  // 1. Gather all active audio clips across audio and video tracks
  const activeAudioClips: ActiveAudioClip[] = [];
  for (const track of tracks) {
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item && (item.kind === "audio" || (item.kind === "video" && track.kind === "video"))) {
          const src = resolveMediaStreamUrl(item, preparedPreviews, mediaServerPort);
          if (src) {
            const currentTime = Math.max(0, (clip.source_in + playhead - clip.timeline_start) / TICKS_PER_SECOND);
            activeAudioClips.push({
              track,
              clip,
              media: item,
              src,
              currentTime,
              gainDb: track.gain_db ?? 0,
            });
          }
        }
      }
    }
  }

  // 2. Identify active title overlay
  let activeTitleClip: { track: Track; clip: Clip; media: MediaAsset; title: TitleProperties } | null = null;
  for (const track of tracks) {
    if (track.kind !== "video") continue;
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item?.kind === "title" && item.title) {
          activeTitleClip = { track, clip, media: item, title: item.title };
          break;
        }
      }
    }
    if (activeTitleClip) break;
  }

  // 3. Find primary video track and visual clips
  // We look through video tracks from top to bottom
  const videoTracks = tracks.filter((t) => t.kind === "video");
  let activeTrack: Track | null = null;
  let activeClip: Clip | null = null;
  let activeMedia: MediaAsset | null = null;

  for (const track of videoTracks) {
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item && (item.kind === "video" || item.kind === "image")) {
          activeTrack = track;
          activeClip = clip;
          activeMedia = item;
          break;
        }
      }
    }
    if (activeClip) break;
  }

  // 4. Check for active transition
  let activeTransition: Transition | null = null;
  let transitionTrack: Track | null = null;

  for (const track of videoTracks) {
    if (!track.transitions) continue;
    for (const trans of track.transitions) {
      const halfDur = Math.floor(trans.duration / 2);
      if (playhead >= trans.cut_at - halfDur && playhead <= trans.cut_at + halfDur) {
        activeTransition = trans;
        transitionTrack = track;
        break;
      }
    }
    if (activeTransition) break;
  }

  // Helper to extract vignette params from clip
  const getClipVignette = (clip: Clip | null): { intensity: number; radius: number; softness: number } | null => {
    if (!clip || !clip.effects) return null;
    const vig = clip.effects.find((e) => e.enabled && e.kind === "vignette");
    if (!vig) return null;
    const p = vig.params as VignetteParams;
    return {
      intensity: p.intensity ?? 0.5,
      radius: p.radius ?? 0.8,
      softness: p.softness ?? 0.5,
    };
  };

  // Helper to extract chroma key params from clip
  const getClipChromaKey = (clip: Clip | null): ChromaKeyParams | null => {
    if (!clip || !clip.effects) return null;
    const chroma = clip.effects.find((e) => e.enabled && e.kind === "chroma_key");
    if (!chroma) return null;
    return chroma.params as ChromaKeyParams;
  };

  // Visual transform and composable effect stack styling helper
  const computeVisualStyles = (clip: Clip | null, isPrimary = true): CSSProperties => {
    if (!clip) return {};
    const clipTime = playhead - clip.timeline_start;

    let brightness = 1;
    let contrast = 1;
    let saturate = 1;
    let hueRotate = 0;
    let blurPx = 0;
    let cropInset: string | null = null;

    if (clip.effects && clip.effects.length > 0) {
      for (const eff of clip.effects) {
        if (!eff.enabled) continue;
        if (eff.kind === "color_correction") {
          const p = eff.params as ColorCorrectionParams;
          brightness *= 2 ** (p.exposure ?? 0);
          contrast *= p.contrast ?? 1;
          saturate *= p.saturation ?? 1;
          hueRotate += (p.hue ?? 0) * 180;
        } else if (eff.kind === "gaussian_blur") {
          const p = eff.params as GaussianBlurParams;
          blurPx += (p.sigma ?? 0) * 2;
        } else if (eff.kind === "crop") {
          const p = eff.params as CropParams;
          cropInset = `inset(${p.top ?? 0}% ${p.right ?? 0}% ${p.bottom ?? 0}% ${p.left ?? 0}%)`;
        }
      }
    } else {
      const color = clip.color || { exposure: 0, contrast: 1, saturation: 1 };
      brightness = 2 ** (color.exposure ?? 0);
      contrast = color.contrast ?? 1;
      saturate = color.saturation ?? 1;
    }

    const filterParts: string[] = [];
    if (Math.abs(brightness - 1) > 0.001) filterParts.push(`brightness(${brightness.toFixed(3)})`);
    if (Math.abs(contrast - 1) > 0.001) filterParts.push(`contrast(${contrast.toFixed(3)})`);
    if (Math.abs(saturate - 1) > 0.001) filterParts.push(`saturate(${saturate.toFixed(3)})`);
    if (Math.abs(hueRotate) > 0.1) filterParts.push(`hue-rotate(${hueRotate.toFixed(1)}deg)`);
    if (blurPx > 0) filterParts.push(`blur(${blurPx.toFixed(1)}px)`);

    const posX = interpolateKeyframe(clip, "position_x", clipTime, 0);
    const posY = interpolateKeyframe(clip, "position_y", clipTime, 0);
    const scale = interpolateKeyframe(clip, "scale", clipTime, 1);
    const rotation = interpolateKeyframe(clip, "rotation", clipTime, 0);
    const opacity = interpolateKeyframe(clip, "opacity", clipTime, 1);

    const transform = `translate(${(posX / projectWidth) * 100}%, ${(posY / projectHeight) * 100}%) scale(${scale}) rotate(${rotation}deg)`;

    return {
      filter: filterParts.length > 0 ? filterParts.join(" ") : undefined,
      clipPath: cropInset ?? undefined,
      transform: isPrimary ? transform : undefined,
      opacity,
      transformOrigin: "center",
    };
  };

  // Case A: Inside an active Transition boundary
  if (activeTransition && transitionTrack) {
    // Locate the outgoing clip (ending at cut_at) and incoming clip (starting at cut_at)
    const outgoing = transitionTrack.clips.find(
      (c) => Math.abs(c.timeline_start + c.duration - activeTransition!.cut_at) < TICKS_PER_SECOND / 10
    );
    const incoming = transitionTrack.clips.find(
      (c) => Math.abs(c.timeline_start - activeTransition!.cut_at) < TICKS_PER_SECOND / 10
    );

    const outMedia = outgoing ? mediaList.find((m) => m.id === outgoing.media_id) : null;
    const inMedia = incoming ? mediaList.find((m) => m.id === incoming.media_id) : null;

    const outSrc = resolveMediaStreamUrl(outMedia, preparedPreviews, mediaServerPort);
    const inSrc = resolveMediaStreamUrl(inMedia, preparedPreviews, mediaServerPort);

    const outTime = outgoing ? Math.max(0, (outgoing.source_in + playhead - outgoing.timeline_start) / TICKS_PER_SECOND) : 0;
    const inTime = incoming ? Math.max(0, (incoming.source_in + playhead - incoming.timeline_start) / TICKS_PER_SECOND) : 0;

    const blend = computeTransitionBlend(activeTransition, playhead);
    const outBaseStyle = computeVisualStyles(outgoing ?? null);
    const inBaseStyle = computeVisualStyles(incoming ?? null);

    return {
      playhead,
      sequenceDuration,
      deckA: {
        mediaId: outMedia?.id ?? null,
        src: outSrc,
        currentTime: outTime,
        visible: Boolean(outSrc),
        style: { ...outBaseStyle, ...blend.deckAStyle },
        vignette: getClipVignette(outgoing ?? null),
        chromaKey: getClipChromaKey(outgoing ?? null),
        clip: outgoing ?? null,
        track: transitionTrack,
        isPreload: false,
      },
      deckB: {
        mediaId: inMedia?.id ?? null,
        src: inSrc,
        currentTime: inTime,
        visible: Boolean(inSrc),
        style: { ...inBaseStyle, ...blend.deckBStyle },
        vignette: getClipVignette(incoming ?? null),
        chromaKey: getClipChromaKey(incoming ?? null),
        clip: incoming ?? null,
        track: transitionTrack,
        isPreload: false,
      },
      activeDeck: blend.progress < 0.5 ? "A" : "B",
      inTransition: true,
      transitionKind: activeTransition.kind,
      activeAudioClips,
      activeTitleClip,
      hasVideoMedia: Boolean(outSrc || inSrc),
    };
  }

  // Case B: Standard Single Active Clip with Next Clip Preloaded on Staging Deck
  if (activeClip && activeMedia && activeTrack) {
    const activeSrc = resolveMediaStreamUrl(activeMedia, preparedPreviews, mediaServerPort);
    const activeTime = Math.max(0, (activeClip.source_in + playhead - activeClip.timeline_start) / TICKS_PER_SECOND);
    const activeStyle = computeVisualStyles(activeClip);

    // Look ahead for the next clip on the same video track
    const nextClip = activeTrack.clips.find((c) => c.timeline_start >= activeClip!.timeline_start + activeClip!.duration);
    const nextMedia = nextClip ? mediaList.find((m) => m.id === nextClip.media_id) : null;
    const nextSrc = resolveMediaStreamUrl(nextMedia, preparedPreviews, mediaServerPort);
    const nextTime = nextClip ? Math.max(0, nextClip.source_in / TICKS_PER_SECOND) : 0;

    // Deck A is active with current clip, Deck B stages next clip for seamless cut handoff
    return {
      playhead,
      sequenceDuration,
      deckA: {
        mediaId: activeMedia.id,
        src: activeSrc,
        currentTime: activeTime,
        visible: true,
        style: { ...activeStyle, opacity: 1 },
        vignette: getClipVignette(activeClip),
        chromaKey: getClipChromaKey(activeClip),
        clip: activeClip,
        track: activeTrack,
        isPreload: false,
      },
      deckB: {
        mediaId: nextMedia?.id ?? null,
        src: nextSrc,
        currentTime: nextTime,
        visible: false, // Preloaded, hidden until cut point
        style: { opacity: 0, pointerEvents: "none" },
        vignette: getClipVignette(nextClip ?? null),
        chromaKey: getClipChromaKey(nextClip ?? null),
        clip: nextClip ?? null,
        track: activeTrack,
        isPreload: true,
      },
      activeDeck: "A",
      inTransition: false,
      transitionKind: null,
      activeAudioClips,
      activeTitleClip,
      hasVideoMedia: true,
    };
  }

  // Case C: Timeline Gap (No video clip at playhead)
  // Look ahead for the upcoming video clip across tracks to preload it
  let upcomingClip: Clip | null = null;
  let upcomingTrack: Track | null = null;

  for (const track of videoTracks) {
    for (const clip of track.clips) {
      if (clip.timeline_start > playhead) {
        if (!upcomingClip || clip.timeline_start < upcomingClip.timeline_start) {
          upcomingClip = clip;
          upcomingTrack = track;
        }
      }
    }
  }

  const upcomingMedia = upcomingClip ? mediaList.find((m) => m.id === upcomingClip.media_id) : null;
  const upcomingSrc = resolveMediaStreamUrl(upcomingMedia, preparedPreviews, mediaServerPort);
  const upcomingTime = upcomingClip ? Math.max(0, upcomingClip.source_in / TICKS_PER_SECOND) : 0;

  return {
    playhead,
    sequenceDuration,
    deckA: {
      mediaId: null,
      src: "",
      currentTime: 0,
      visible: false,
      style: { opacity: 0 },
      vignette: null,
      chromaKey: null,
      clip: null,
      track: null,
      isPreload: false,
    },
    deckB: {
      mediaId: upcomingMedia?.id ?? null,
      src: upcomingSrc,
      currentTime: upcomingTime,
      visible: false, // Preloaded during gap, ready for cut arrival
      style: { opacity: 0, pointerEvents: "none" },
      vignette: getClipVignette(upcomingClip ?? null),
      chromaKey: getClipChromaKey(upcomingClip ?? null),
      clip: upcomingClip,
      track: upcomingTrack,
      isPreload: true,
    },
    activeDeck: "A",
    inTransition: false,
    transitionKind: null,
    activeAudioClips,
    activeTitleClip,
    hasVideoMedia: false,
  };
}
