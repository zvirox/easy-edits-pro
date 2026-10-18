import type { CSSProperties } from "react";
import type { ColorCorrectionParams, GaussianBlurParams, CropParams, VignetteParams, ChromaKeyParams, LutParams, Clip } from "./types.ts";
import { TICKS_PER_SECOND } from "./constants.ts";
import { interpolateKeyframe } from "./interpolation.ts";

// Helper to extract vignette params from clip
export function getClipVignette(clip: Clip | null): { intensity: number; radius: number; softness: number } | null {
  if (!clip || !clip.effects) return null;
  const vig = clip.effects.find((e) => e.enabled && e.kind === "vignette");
  if (!vig) return null;
  const p = vig.params as VignetteParams;
  return {
    intensity: p.intensity ?? 0.5,
    radius: p.radius ?? 0.8,
    softness: p.softness ?? 0.5,
  };
}

// Helper to extract chroma key params from clip
export function getClipChromaKey(clip: Clip | null): ChromaKeyParams | null {
  if (!clip || !clip.effects) return null;
  const chroma = clip.effects.find((e) => e.enabled && e.kind === "chroma_key");
  if (!chroma) return null;
  return chroma.params as ChromaKeyParams;
}

// Visual transform and composable effect stack styling helper
export function computeVisualStyles(clip: Clip | null, playhead: number, projectWidth: number, projectHeight: number, isPrimary = true): CSSProperties {
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
}
