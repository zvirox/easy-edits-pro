import type { Transition, Variants } from "motion/react";
import { useSyncExternalStore } from "react";

/**
 * Shared Phase 3 motion presets.
 *
 * Only compositor-friendly properties live here: opacity, transform and
 * filter. Layout properties (width, height, inset, margin, padding) are banned
 * by scripts/check-styles.mjs.
 */
export const easeOut: [number, number, number, number] = [0.22, 1, 0.36, 1];
export const motionFast = 0.12;
export const motionMed = 0.22;
export const motionSlow = 0.36;

export const fadeTransition: Transition = { duration: motionFast, ease: easeOut };
export const popTransition: Transition = { duration: motionMed, ease: easeOut };
export const slideTransition: Transition = { duration: motionSlow, ease: easeOut };

export const backdropVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
};

export const dialogVariants: Variants = {
  initial: { opacity: 0, scale: 0.96, y: 8 },
  animate: { opacity: 1, scale: 1, y: 0 },
  exit: { opacity: 0, scale: 0.98, y: 4 },
};

export const menuVariants: Variants = {
  initial: { opacity: 0, scale: 0.98, y: -4 },
  animate: { opacity: 1, scale: 1, y: 0 },
  exit: { opacity: 0, scale: 0.98, y: -2 },
};

export const drawerVariants: Variants = {
  initial: { opacity: 0, x: 48 },
  animate: { opacity: 1, x: 0 },
  exit: { opacity: 0, x: 32 },
};

export const panelVariants: Variants = {
  initial: { opacity: 0, scale: 0.99 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.99 },
};

export const toastVariants: Variants = {
  initial: { opacity: 0, y: 12, scale: 0.98 },
  animate: { opacity: 1, y: 0, scale: 1 },
  exit: { opacity: 0, y: 8, scale: 0.98 },
};

export interface MotionPreset {
  reduced: boolean;
  initial: false | "initial";
  transition: Transition;
}

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

let reducedMotionOverride: boolean | null = null;
const reducedMotionListeners = new Set<() => void>();

/**
 * Preferences can force motion on or off. `null` hands control back to the
 * operating system media query.
 */
export function setReducedMotionOverride(value: boolean | null): void {
  if (reducedMotionOverride === value) return;
  reducedMotionOverride = value;
  for (const listener of reducedMotionListeners) listener();
}

function subscribeReducedMotion(onChange: () => void) {
  const query = window.matchMedia(REDUCED_MOTION_QUERY);
  query.addEventListener("change", onChange);
  reducedMotionListeners.add(onChange);
  return () => {
    query.removeEventListener("change", onChange);
    reducedMotionListeners.delete(onChange);
  };
}

function reducedMotionSnapshot(): boolean {
  if (reducedMotionOverride !== null) return reducedMotionOverride;
  return window.matchMedia(REDUCED_MOTION_QUERY).matches;
}

/**
 * Reactive reduced-motion preference. Motion/react's own useReducedMotion
 * snapshots the OS setting once at mount and never updates, so we read the
 * media query directly and re-render on changes.
 */
export function useReducedMotionPreference(): boolean {
  return useSyncExternalStore(subscribeReducedMotion, reducedMotionSnapshot, () => false);
}

/**
 * Returns the props every presence animation needs. When the user prefers
 * reduced motion the element still mounts/unmounts, but it does so instantly.
 * A zero-duration transition also collapses the opacity run, which Motion's
 * internal reduced-motion handling does not do on its own.
 */
export function useMotionPreset(transition: Transition = popTransition): MotionPreset {
  const reduced = useReducedMotionPreference();
  return {
    reduced,
    initial: reduced ? false : "initial",
    transition: reduced ? { duration: 0 } : transition,
  };
}
