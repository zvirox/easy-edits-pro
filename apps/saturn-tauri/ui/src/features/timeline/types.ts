import type { IconName } from "../../components/Icon";
import type { TimelineTool } from "../../app/projectTypes";

export type { TimelineTool };
export { TRACK_LABEL_WIDTH, TICKS_PER_SECOND, getTrackGaps } from "../../app/projectTypes";

export type TimelineToolGroup = "Select" | "Trim" | "Cut & Key" | "Navigate" | "Graphics";

export const timelineTools: {
  id: TimelineTool;
  icon: IconName;
  label: string;
  shortLabel: string;
  shortcut: string;
  group: TimelineToolGroup;
}[] = [
  { id: "select", icon: "select", label: "Selection Tool", shortLabel: "Select", shortcut: "V", group: "Select" },
  { id: "trackForward", icon: "trackForward", label: "Track Select Forward", shortLabel: "Track", shortcut: "A", group: "Select" },
  { id: "trackBackward", icon: "trackBackward", label: "Track Select Backward", shortLabel: "Back", shortcut: "Shift+A", group: "Select" },
  { id: "ripple", icon: "ripple", label: "Ripple Edit Tool", shortLabel: "Ripple", shortcut: "B", group: "Trim" },
  { id: "rolling", icon: "rolling", label: "Rolling Edit Tool", shortLabel: "Rolling", shortcut: "N", group: "Trim" },
  { id: "slip", icon: "slip", label: "Slip Tool", shortLabel: "Slip", shortcut: "Y", group: "Trim" },
  { id: "slide", icon: "slide", label: "Slide Edit Tool", shortLabel: "Slide", shortcut: "U", group: "Trim" },
  { id: "rateStretch", icon: "rateStretch", label: "Rate Stretch Tool", shortLabel: "Stretch", shortcut: "X", group: "Trim" },
  { id: "remix", icon: "remix", label: "Remix Tool", shortLabel: "Remix", shortcut: "Shift+R", group: "Trim" },
  { id: "razor", icon: "razor", label: "Razor Tool", shortLabel: "Razor", shortcut: "C", group: "Cut & Key" },
  { id: "pen", icon: "pen", label: "Keyframe Pen Tool", shortLabel: "Pen", shortcut: "P", group: "Cut & Key" },
  { id: "hand", icon: "hand", label: "Hand Pan Tool", shortLabel: "Hand", shortcut: "H", group: "Navigate" },
  { id: "zoom", icon: "zoomIn", label: "Zoom Tool", shortLabel: "Zoom", shortcut: "Z", group: "Navigate" },
  { id: "rectangle", icon: "rectangle", label: "Rectangle Tool", shortLabel: "Rect", shortcut: "R", group: "Graphics" },
  { id: "ellipse", icon: "ellipse", label: "Ellipse Tool", shortLabel: "Ellipse", shortcut: "E", group: "Graphics" },
  { id: "type", icon: "type", label: "Type Tool", shortLabel: "Type", shortcut: "T", group: "Graphics" },
  { id: "verticalType", icon: "verticalType", label: "Vertical Type Tool", shortLabel: "V-Type", shortcut: "Shift+T", group: "Graphics" },
];
