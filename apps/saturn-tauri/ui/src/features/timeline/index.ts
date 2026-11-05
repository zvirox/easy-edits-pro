// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
export type { Clip, Track, Sequence, Transition } from "../../app/projectTypes";
export {
  TimelineSection,
  type TimelineSectionProps,
  timelineTools,
  getTrackGaps,
  TRACK_LABEL_WIDTH,
  TICKS_PER_SECOND,
  type TimelineTool,
} from "./TimelineSection";
export { TimelineToolbar, type TimelineToolbarProps, TimelineToolRail, type TimelineToolRailProps } from "./TimelineToolbar";
export { SequenceTabs, type SequenceTab, type SequenceTabsProps } from "./SequenceTabs";
export {
  ClipContextMenu,
  type ClipContextMenuProps,
  type ClipMenuCommand,
} from "./ClipContextMenu";
export { TrimMonitorPanel, type TrimMonitorPanelProps } from "./TrimMonitorPanel";
