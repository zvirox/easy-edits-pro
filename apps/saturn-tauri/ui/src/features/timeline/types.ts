import type { Clip } from "../../app/projectTypes";
import type { IconName } from "../../components/Icon";

export type TimelineTool =
  | "select"
  | "trackForward"
  | "ripple"
  | "rolling"
  | "razor"
  | "slip"
  | "slide"
  | "rateStretch"
  | "pen"
  | "hand"
  | "zoom";

export const timelineTools: {
  id: TimelineTool;
  icon: IconName;
  label: string;
  shortLabel: string;
  shortcut: string;
}[] = [
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
