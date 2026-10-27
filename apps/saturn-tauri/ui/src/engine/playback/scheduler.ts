import type { Transition, Clip, Track, TitleProperties, MediaAsset, SubtitleItem, ProjectDocument, TransitionBlend, ActiveAudioClip, TimelinePlaybackSnapshot } from "./types.ts";
import { TICKS_PER_SECOND } from "./constants.ts";
import { computeVisualStyles, getClipVignette, getClipChromaKey } from "./effects.ts";
import { resolveMediaStreamUrl } from "./mediaUrl.ts";

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

// FC Enable: disabled clips stay on the timeline but play nothing, so every
// downstream gather (audio, title, visual) sees them filtered out here.
const tracks = project.project.sequence.tracks.map((track) => ({
  ...track,
  clips: track.clips.filter((clip) => clip.enabled !== false),
}));
const projectWidth = project.project.settings.width || 1920;
const projectHeight = project.project.settings.height || 1080;
const subtitles = (project.project.sequence.caption_tracks ?? [])
  // Hidden caption tracks keep their cues but stay out of the program overlay.
  .filter((track) => track.visible !== false)
  .flatMap((track) => track.cues);
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
  const outBaseStyle = computeVisualStyles(outgoing ?? null, playhead, projectWidth, projectHeight);
  const inBaseStyle = computeVisualStyles(incoming ?? null, playhead, projectWidth, projectHeight);

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
  const activeStyle = computeVisualStyles(activeClip, playhead, projectWidth, projectHeight);

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
