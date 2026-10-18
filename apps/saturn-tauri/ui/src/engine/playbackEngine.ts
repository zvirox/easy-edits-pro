import type { CSSProperties } from "react";
export { EbuR128WebMeter, AudioClockMaster, audioClockMaster } from "./playback/index.ts";
export type { TransitionKind, Transition, KeyframeProperty, InterpolationKind, Keyframe, ColorWheelParam, ColorWheels, CurvePoint, RgbCurves, HslQualifier, TrackGroup, ColorAdjustments, MultiCamSyncMode, MultiCamAngle, MultiCamClip, TrackingMode, TrackPoint, MotionTrackingData, BusTarget, AutomationMode, AutomationPoint, TrackAutomation, AudioPluginInstance, AudioBus, EffectKind, ColorCorrectionParams, GaussianBlurParams, CropParams, VignetteParams, ChromaKeyParams, LutParams, CustomShaderParams, EffectPayload, EffectInstance, CompositingNodeKind, CompositingNode, NodeConnection, RotoSplinePoint, RotoSplineMask, NodeGraph, SpeedRampPoint, SpeedRamp, MotionBlurParams, LockTargetKind, UserLock, TimelineChangeEntry, CollaborationState, Clip, TrackAudioProcessing, Track, TitleProperties, Sequence, MediaBin, MediaAsset, SubtitleItem, ProjectDocument, LoudnessStandard, LoudnessReport, TransitionBlend, DeckConfig, ActiveAudioClip, TimelinePlaybackSnapshot, SpeechWord, SpeechSegment, SilenceInterval, RippleDeleteReport, PresentationMetrics, InteropFormat } from "./playback/types.ts";
import { buildMediaStreamUrl } from "../utils/mediaUrl.ts";
import type {   Transition, KeyframeProperty, ColorCorrectionParams, GaussianBlurParams, CropParams, VignetteParams, ChromaKeyParams, LutParams, Clip, Track, TitleProperties, MediaAsset, SubtitleItem, ProjectDocument, TransitionBlend, ActiveAudioClip , TimelinePlaybackSnapshot } from "./playback/types.ts";

import { TICKS_PER_SECOND } from "./playback/constants.ts";
export { TICKS_PER_SECOND } from "./playback/constants.ts";




























































/**
 * Evaluates a cubic bezier curve at parameter t in [0, 1] with control points (0,0), (x1, y1), (x2, y2), (1,1).
 */
export function solveCubicBezier(t: number, x1: number, y1: number, x2: number, y2: number): number {
  const clampedT = Math.max(0, Math.min(1, t));
  if (clampedT <= 0) return 0;
  if (clampedT >= 1) return 1;

  // B(s) = 3(1-s)^2*s*P1 + 3(1-s)*s^2*P2 + s^3
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;

  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;

  const sampleCurveX = (s: number) => ((ax * s + bx) * s + cx) * s;
  const sampleCurveY = (s: number) => ((ay * s + by) * s + cy) * s;
  const sampleCurveDerivativeX = (s: number) => (3 * ax * s + 2 * bx) * s + cx;

  // Newton-Raphson iteration
  let s = clampedT;
  for (let i = 0; i < 8; i++) {
    const x = sampleCurveX(s) - clampedT;
    if (Math.abs(x) < 1e-5) return sampleCurveY(s);
    const d = sampleCurveDerivativeX(s);
    if (Math.abs(d) < 1e-5) break;
    s -= x / d;
  }

  // Binary subdivision fallback
  let lo = 0;
  let hi = 1;
  s = clampedT;
  for (let i = 0; i < 12; i++) {
    const x = sampleCurveX(s);
    if (Math.abs(x - clampedT) < 1e-4) return sampleCurveY(s);
    if (clampedT > x) lo = s;
    else hi = s;
    s = (hi + lo) / 2;
  }

  return sampleCurveY(s);
}

/**
 * Extracts active subtitle item at a specific timeline playhead in ticks.
 */
export function getActiveSubtitle(subtitles: SubtitleItem[] | undefined, playheadTicks: number): SubtitleItem | null {
  if (!subtitles || subtitles.length === 0) return null;
  for (const sub of subtitles) {
    if (playheadTicks >= sub.start_ticks && playheadTicks < sub.end_ticks) {
      return sub;
    }
  }
  return null;
}

/**
 * Computes an animated keyframe property value with bezier & ease curve interpolation.
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
  const ratio = Math.max(0, Math.min(1, (clipTimeTicks - prev.time) / span));

  const interp = prev.interpolation ?? "linear";
  let progress = ratio;

  switch (interp) {
    case "linear":
      progress = ratio;
      break;
    case "ease_in":
      progress = solveCubicBezier(ratio, 0.42, 0.0, 1.0, 1.0);
      break;
    case "ease_out":
      progress = solveCubicBezier(ratio, 0.0, 0.0, 0.58, 1.0);
      break;
    case "ease_in_out":
      progress = solveCubicBezier(ratio, 0.42, 0.0, 0.58, 1.0);
      break;
    case "bezier": {
      const x1 = prev.handle_out_x ?? 0.33;
      const y1 = prev.handle_out_y ?? 0.0;
      const x2 = next.handle_in_x ?? 0.67;
      const y2 = next.handle_in_y ?? 1.0;
      progress = solveCubicBezier(ratio, x1, y1, x2, y2);
      break;
    }
    default:
      progress = ratio;
      break;
  }

  return prev.value + (next.value - prev.value) * progress;
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




/**
 * Resolves the URL for a media asset, preferring prepared WebM/VP8 proxies for video.
 */
export function resolveMediaStreamUrl(
  mediaAsset: MediaAsset | null | undefined,
  preparedPreviews: Record<number, string>,
  mediaServerPort: number,
  mediaServerToken?: string
): string {
  if (!mediaAsset || mediaServerPort <= 0) return "";
  const targetPath = preparedPreviews[mediaAsset.id] || mediaAsset.path;
  return buildMediaStreamUrl(targetPath, mediaServerPort, mediaServerToken);
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
  mediaServerPort: number,
  mediaServerToken?: string
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
    activeSubtitle: null,
    hasVideoMedia: false,
  };

  if (!project) return emptyResult;

  const tracks = project.project.sequence.tracks;
  const projectWidth = project.project.settings.width || 1920;
  const projectHeight = project.project.settings.height || 1080;
  const subtitles = project.project.sequence.subtitles;
  const activeSubtitle = getActiveSubtitle(subtitles, playhead);

  // Compute sequence duration across all tracks
  let maxEnd = 0;
  for (const track of tracks) {
    for (const clip of track.clips) {
      const end = clip.timeline_start + clip.duration;
      if (end > maxEnd) maxEnd = end;
    }
  }
  const sequenceDuration = maxEnd;

  // 1. Gather all active audio clips across audio and video tracks (including compound sequences)
  const activeAudioClips: ActiveAudioClip[] = [];
  for (const track of tracks) {
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item && (item.kind === "compound_sequence" || clip.compound_sequence_id != null)) {
          const nestedSeq = item.nested_sequence || project.project.sequences?.find((s) => s.id === clip.compound_sequence_id);
          if (nestedSeq) {
            const speed = clip.speed ?? 1.0;
            const localPlayhead = Math.max(0, clip.source_in + Math.round((playhead - clip.timeline_start) * speed));
            for (const nTrack of nestedSeq.tracks) {
              for (const nClip of nTrack.clips) {
                if (localPlayhead >= nClip.timeline_start && localPlayhead < nClip.timeline_start + nClip.duration) {
                  const nItem = mediaList.find((m) => m.id === nClip.media_id);
                  if (nItem && (nItem.kind === "audio" || (nItem.kind === "video" && nTrack.kind === "video"))) {
                    // Always stream audio from the original media path, not the video-only transcode proxy
                    const src = resolveMediaStreamUrl(nItem, {}, mediaServerPort, mediaServerToken);
                    if (src) {
                      const currentTime = Math.max(0, (nClip.source_in + (localPlayhead - nClip.timeline_start) * (nClip.speed ?? 1.0)) / TICKS_PER_SECOND);
                      activeAudioClips.push({
                        track: nTrack,
                        clip: nClip,
                        media: nItem,
                        src,
                        currentTime,
                        gainDb: (track.gain_db ?? 0) + (nTrack.gain_db ?? 0),
                      });
                    }
                  }
                }
              }
            }
          }
        } else if (item && (item.kind === "audio" || item.kind === "video")) {
          // Always stream audio from the original media path, not the video-only transcode proxy
          const src = resolveMediaStreamUrl(item, {}, mediaServerPort, mediaServerToken);
          if (src) {
            const speed = clip.speed ?? 1.0;
            const currentTime = Math.max(0, (clip.source_in + (playhead - clip.timeline_start) * speed) / TICKS_PER_SECOND);
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
        } else if (item && (item.kind === "compound_sequence" || clip.compound_sequence_id != null)) {
          const nestedSeq = item.nested_sequence || project.project.sequences?.find((s) => s.id === clip.compound_sequence_id);
          if (nestedSeq) {
            const speed = clip.speed ?? 1.0;
            const localPlayhead = Math.max(0, clip.source_in + Math.round((playhead - clip.timeline_start) * speed));
            for (const nTrack of nestedSeq.tracks) {
              if (nTrack.kind !== "video") continue;
              for (const nClip of nTrack.clips) {
                if (localPlayhead >= nClip.timeline_start && localPlayhead < nClip.timeline_start + nClip.duration) {
                  const nItem = mediaList.find((m) => m.id === nClip.media_id);
                  if (nItem?.kind === "title" && nItem.title) {
                    activeTitleClip = { track: nTrack, clip: nClip, media: nItem, title: nItem.title };
                    break;
                  }
                }
              }
              if (activeTitleClip) break;
            }
          }
        }
      }
    }
    if (activeTitleClip) break;
  }

  // 3. Find primary video track and visual clips (including compound sequences)
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
        } else if (item && (item.kind === "compound_sequence" || clip.compound_sequence_id != null)) {
          const nestedSeq = item.nested_sequence || project.project.sequences?.find((s) => s.id === clip.compound_sequence_id);
          if (nestedSeq) {
            const speed = clip.speed ?? 1.0;
            const localPlayhead = Math.max(0, clip.source_in + Math.round((playhead - clip.timeline_start) * speed));
            for (const nTrack of nestedSeq.tracks.filter((t) => t.kind === "video")) {
              for (const nClip of nTrack.clips) {
                if (localPlayhead >= nClip.timeline_start && localPlayhead < nClip.timeline_start + nClip.duration) {
                  const nItem = mediaList.find((m) => m.id === nClip.media_id);
                  if (nItem && (nItem.kind === "video" || nItem.kind === "image")) {
                    activeTrack = track;
                    activeClip = {
                      ...nClip,
                      timeline_start: clip.timeline_start,
                      duration: clip.duration,
                      source_in: Math.max(0, nClip.source_in + (localPlayhead - nClip.timeline_start)),
                    };
                    activeMedia = nItem;
                    break;
                  }
                }
              }
              if (activeClip) break;
            }
          }
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
        } else if (eff.kind === "lut") {
          const p = eff.params as LutParams;
          const blend = Math.max(0, Math.min(1, p.intensity ?? 1.0));
          const preset = p.preset_id ?? "teal_orange";
          if (blend > 0) {
            switch (preset) {
              case "teal_orange":
                contrast *= 1 + 0.15 * blend;
                saturate *= 1 + 0.25 * blend;
                hueRotate += -12 * blend;
                break;
              case "vintage":
                contrast *= 1 - 0.08 * blend;
                brightness *= 1 + 0.05 * blend;
                saturate *= 1 - 0.15 * blend;
                hueRotate += 8 * blend;
                break;
              case "film_noir":
                saturate *= 1 - 1.0 * blend;
                contrast *= 1 + 0.35 * blend;
                break;
              case "rec709":
                contrast *= 1 + 0.12 * blend;
                saturate *= 1 + 0.12 * blend;
                break;
              default:
                contrast *= 1 + 0.1 * blend;
                saturate *= 1 + 0.1 * blend;
                break;
            }
          }
        }
      }
    } else {
      const color = clip.color || { exposure: 0, contrast: 1, saturation: 1 };
      brightness = 2 ** (color.exposure ?? 0);
      contrast = color.contrast ?? 1;
      saturate = color.saturation ?? 1;
      hueRotate += (color.hue ?? 0) * 180;
    }

    // Color Wheels (Lift, Gamma, Gain, Offset)
    const wheels = clip.color?.wheels;
    if (wheels) {
      const liftY = wheels.lift?.luma ?? 0;
      const gammaY = wheels.gamma?.luma ?? 0;
      const gainY = wheels.gain?.luma ?? 0;
      const offsetY = wheels.offset?.luma ?? 0;

      brightness *= 1 + (liftY * 0.25 + gainY * 0.35 + offsetY * 0.3);
      contrast *= 1 + (gainY * 0.2 - liftY * 0.15 + gammaY * 0.1);

      const rShift = (wheels.lift?.red ?? 0) * 0.15 + (wheels.gain?.red ?? 0) * 0.2 + (wheels.offset?.red ?? 0) * 0.1;
      const bShift = (wheels.lift?.blue ?? 0) * 0.15 + (wheels.gain?.blue ?? 0) * 0.2 + (wheels.offset?.blue ?? 0) * 0.1;
      const tintDeg = (rShift - bShift) * 40;
      hueRotate += tintDeg;
    }

    // Motion Blur (dynamic shutter angle simulation)
    if (clip.motion_blur?.enabled && (clip.motion_blur.shutter_angle_deg ?? 180) > 0) {
      const shutter = clip.motion_blur.shutter_angle_deg ?? 180;
      const motionBlurPx = (shutter / 360) * 1.5;
      if (motionBlurPx > 0.1) {
        blurPx += motionBlurPx;
      }
    }

    // Fusion Node Graph roto-spline mask polygon
    if (clip.node_graph?.masks && clip.node_graph.masks.length > 0) {
      const activeMask = clip.node_graph.masks[0];
      if (activeMask.points && activeMask.points.length >= 3) {
        const polyCoords = activeMask.points.map((p) => `${p.x}% ${p.y}%`).join(", ");
        cropInset = `polygon(${polyCoords})`;
      }
    }

    const filterParts: string[] = [];
    if (Math.abs(brightness - 1) > 0.001) filterParts.push(`brightness(${brightness.toFixed(3)})`);
    if (Math.abs(contrast - 1) > 0.001) filterParts.push(`contrast(${contrast.toFixed(3)})`);
    if (Math.abs(saturate - 1) > 0.001) filterParts.push(`saturate(${saturate.toFixed(3)})`);
    if (Math.abs(hueRotate) > 0.1) filterParts.push(`hue-rotate(${hueRotate.toFixed(1)}deg)`);
    if (blurPx > 0) filterParts.push(`blur(${blurPx.toFixed(1)}px)`);

    let stabX = 0;
    let stabY = 0;
    if (clip.tracking?.points && clip.tracking.points.length > 0) {
      if (clip.tracking.mode === "stabilize") {
        const pts = clip.tracking.points;
        const smoothness = clip.tracking.smoothness ?? 0.5;
        const pt = pts.find((p) => Math.abs(p.time - clipTime) < TICKS_PER_SECOND / 10) || pts[0];
        if (pt) {
          stabX = (0.5 - pt.x) * projectWidth * smoothness;
          stabY = (0.5 - pt.y) * projectHeight * smoothness;
        }
      }
    }

    const posX = interpolateKeyframe(clip, "position_x", clipTime, 0) + stabX;
    const posY = interpolateKeyframe(clip, "position_y", clipTime, 0) + stabY;
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

    const outSrc = resolveMediaStreamUrl(outMedia, preparedPreviews, mediaServerPort, mediaServerToken);
    const inSrc = resolveMediaStreamUrl(inMedia, preparedPreviews, mediaServerPort, mediaServerToken);

    const outSpeed = outgoing?.speed ?? 1.0;
    const inSpeed = incoming?.speed ?? 1.0;
    const outTime = outgoing ? Math.max(0, (outgoing.source_in + (playhead - outgoing.timeline_start) * outSpeed) / TICKS_PER_SECOND) : 0;
    const inTime = incoming ? Math.max(0, (incoming.source_in + (playhead - incoming.timeline_start) * inSpeed) / TICKS_PER_SECOND) : 0;

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
      activeSubtitle,
      hasVideoMedia: Boolean(outSrc || inSrc),
    };
  }

  // Case B: Standard Single Active Clip with Next Clip Preloaded on Staging Deck
  if (activeClip && activeMedia && activeTrack) {
    const activeSrc = resolveMediaStreamUrl(activeMedia, preparedPreviews, mediaServerPort, mediaServerToken);
    const clipSpeed = activeClip.speed ?? 1.0;
    const activeTime = Math.max(0, (activeClip.source_in + (playhead - activeClip.timeline_start) * clipSpeed) / TICKS_PER_SECOND);
    const activeStyle = computeVisualStyles(activeClip);

    // Look ahead for the next clip on the same video track
    const nextClip = activeTrack.clips.find((c) => c.timeline_start >= activeClip!.timeline_start + activeClip!.duration);
    const nextMedia = nextClip ? mediaList.find((m) => m.id === nextClip.media_id) : null;
    const nextSrc = resolveMediaStreamUrl(nextMedia, preparedPreviews, mediaServerPort, mediaServerToken);
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
      activeSubtitle,
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
  const upcomingSrc = resolveMediaStreamUrl(upcomingMedia, preparedPreviews, mediaServerPort, mediaServerToken);
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
    activeSubtitle,
    hasVideoMedia: false,
  };
}
