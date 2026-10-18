import type { IconName } from "../../components/Icon";
import type { TimelineTool } from "../../app/projectTypes";

export type { TimelineTool };
export { TRACK_LABEL_WIDTH, TICKS_PER_SECOND, getTrackGaps } from "../../app/projectTypes";

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
