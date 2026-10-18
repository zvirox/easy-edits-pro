import type { Dispatch, SetStateAction } from "react";
import { Icon } from "../../components/Icon";
import { formatTime } from "../../utils/formatters";
import type { MediaAsset, ProjectDocument, SelectedClip, TimelineTool } from "../../app/projectTypes";
import type { CommandStatus } from "../../app/ProjectProvider";
import { timelineTools } from "./types";
export type { TimelineTool };
export { timelineTools };

export interface TimelineToolbarProps {
  timelineTool: TimelineTool;
  setTimelineTool: (tool: TimelineTool) => void;
  project: ProjectDocument["project"] | null;
  switchSequence: (seqId: number) => Promise<void>;
  playhead: number;
  busy: boolean;
  addTrackToTimeline: (kind: "video" | "audio") => Promise<void>;
  timelineTrackHeight: "compact" | "normal" | "expanded";
  setTimelineTrackHeight: Dispatch<SetStateAction<"compact" | "normal" | "expanded">>;
  snapEnabled: boolean;
  setSnapEnabled: Dispatch<SetStateAction<boolean>>;
  addSelectedToTimeline: () => Promise<void>;
  selectedMedia: MediaAsset | null;
  addTitleToTimeline: () => Promise<void>;
  addTransitionToTimeline: () => Promise<void>;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  perform: (id: string, params?: unknown) => Promise<unknown>;
  selectedClip: SelectedClip | null;
  splitSelectedClip: () => Promise<void>;
  rippleDeleteSelectedClip: () => Promise<void>;
  removeSelectedClip: () => void;
  selectedClips: SelectedClip[];
  linkSelectedClips: () => Promise<void>;
  unlinkSelectedClips: () => Promise<void>;
  setCompoundClipNameDraft: (name: string) => void;
  setCompoundClipModalOpen: (open: boolean) => void;
  decomposeSelectedCompoundClip: () => Promise<void>;
  invokeMenuCommand: (cmd: string) => Promise<void>;
  zoomTimeline: (factor: number) => void;
  handleCreateTrackGroup: () => Promise<void>;
}

export function TimelineToolbar({
  timelineTool,
  setTimelineTool,
  project,
  switchSequence,
  playhead,
  busy,
  addTrackToTimeline,
  timelineTrackHeight,
  setTimelineTrackHeight,
  snapEnabled,
  setSnapEnabled,
  addSelectedToTimeline,
  selectedMedia,
  addTitleToTimeline,
  addTransitionToTimeline,
  undoStatus,
  redoStatus,
  perform,
  selectedClip,
  splitSelectedClip,
  rippleDeleteSelectedClip,
  removeSelectedClip,
  selectedClips,
  linkSelectedClips,
  unlinkSelectedClips,
  setCompoundClipNameDraft,
  setCompoundClipModalOpen,
  decomposeSelectedCompoundClip,
  invokeMenuCommand,
  zoomTimeline,
  handleCreateTrackGroup,
}: TimelineToolbarProps) {
  return (
    <>
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

      <div className="timeline-toolbar">
        <div className="timeline-toolbar-start">
          <div className="sequence-breadcrumbs">
            <button
              type="button"
              className={`seq-crumb-btn ${(project?.sequence.id ?? 1) === 1 ? "active" : ""}`}
              onClick={() => void switchSequence(1)}
              title="Switch to Master Sequence"
            >
              🎬 {project?.sequences?.find((s) => (s.id ?? 1) === 1)?.name || "Master Sequence"}
            </button>
            {(project?.sequence.id ?? 1) !== 1 && (
              <>
                <span className="seq-crumb-arrow">›</span>
                <span className="seq-crumb-btn active compound-crumb">
                  ⧉ {project?.sequence.name}
                </span>
                <button
                  type="button"
                  className="seq-exit-compound-btn"
                  onClick={() => void switchSequence(1)}
                  title="Return to Master Sequence"
                >
                  ↶ Exit
                </button>
              </>
            )}
            {project?.sequences && project.sequences.length > 1 && (
              <select
                className="seq-dropdown-select"
                value={project.sequence.id ?? 1}
                onChange={(e) => void switchSequence(Number(e.target.value))}
                title="Jump to Sequence"
              >
                {project.sequences.map((s) => (
                  <option key={s.id ?? 1} value={s.id ?? 1}>
                    {(s.id ?? 1) === 1 ? "🎬 " : "⧉ "} {s.name}
                  </option>
                ))}
              </select>
            )}
          </div>
          <code>
            {formatTime(playhead)} /{" "}
            {formatTime(
              Math.max(
                0,
                ...(project?.sequence.tracks ?? []).flatMap((track) =>
                  track.clips.map((clip) => clip.timeline_start + clip.duration)
                )
              )
            )}
          </code>
        </div>
        <div className="timeline-toolbar-tools" aria-label="Timeline actions">
          <button
            className="timeline-icon-button"
            disabled={busy}
            onClick={() => void addTrackToTimeline("video")}
            data-tooltip="Add Video Track (+V)"
            data-tooltip-pos="top"
            aria-label="Add Video Track"
          >
            <span style={{ fontWeight: 700, fontSize: "11px" }}>+V</span>
          </button>
          <button
            className="timeline-icon-button"
            disabled={busy}
            onClick={() => void addTrackToTimeline("audio")}
            data-tooltip="Add Audio Track (+A)"
            data-tooltip-pos="top"
            aria-label="Add Audio Track"
          >
            <span style={{ fontWeight: 700, fontSize: "11px" }}>+A</span>
          </button>
          <span className="toolbar-divider" />
          <div
            className="track-height-selector"
            style={{
              display: "inline-flex",
              gap: "2px",
              background: "var(--bg-input)",
              padding: "2px",
              borderRadius: "4px",
            }}
          >
            <button
              type="button"
              className={`timeline-mini-toggle ${timelineTrackHeight === "compact" ? "active" : ""}`}
              onClick={() => setTimelineTrackHeight("compact")}
              data-tooltip="Compact Track Height (40px)"
              data-tooltip-pos="top"
            >
              ▫
            </button>
            <button
              type="button"
              className={`timeline-mini-toggle ${timelineTrackHeight === "normal" ? "active" : ""}`}
              onClick={() => setTimelineTrackHeight("normal")}
              data-tooltip="Normal Track Height (64px)"
              data-tooltip-pos="top"
            >
              ◽
            </button>
            <button
              type="button"
              className={`timeline-mini-toggle ${timelineTrackHeight === "expanded" ? "active" : ""}`}
              onClick={() => setTimelineTrackHeight("expanded")}
              data-tooltip="Expanded Track Height (88px)"
              data-tooltip-pos="top"
            >
              ◻
            </button>
          </div>
          <span className="toolbar-divider" />
          <button
            className={`timeline-icon-button ${snapEnabled ? "active-toggle" : ""}`}
            aria-pressed={snapEnabled}
            onClick={() => setSnapEnabled((enabled) => !enabled)}
            data-tooltip={`Snap to Frame: ${snapEnabled ? "On" : "Off"} (S)`}
            data-tooltip-pos="top"
            aria-label="Toggle snapping"
          >
            <Icon name="snap" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={!selectedMedia || busy}
            onClick={() => void addSelectedToTimeline()}
            data-tooltip="Add Selected Media to Timeline (Shift+A)"
            data-tooltip-pos="top"
            aria-label="Add selected media"
          >
            <Icon name="add" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={busy}
            onClick={() => void addTitleToTimeline()}
            data-tooltip="Add Title / Lower-Third Graphic (Ctrl+T)"
            data-tooltip-pos="top"
            aria-label="Add Title"
          >
            <Icon name="title" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={busy}
            onClick={() => void addTransitionToTimeline()}
            data-tooltip="Add Transition at Cut (Ctrl+D)"
            data-tooltip-pos="top"
            aria-label="Add Transition"
          >
            <Icon name="transition" />
          </button>
          <span className="toolbar-divider" />
          <button
            className="timeline-icon-button"
            disabled={!undoStatus.enabled || busy}
            onClick={() => void perform("edit.undo")}
            data-tooltip={`Undo: ${undoStatus.reason ?? "Undo"} (Ctrl+Z)`}
            data-tooltip-pos="top"
            aria-label="Undo"
          >
            <Icon name="undo" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={!redoStatus.enabled || busy}
            onClick={() => void perform("edit.redo")}
            data-tooltip={`Redo: ${redoStatus.reason ?? "Redo"} (Ctrl+Y)`}
            data-tooltip-pos="top"
            aria-label="Redo"
          >
            <Icon name="redo" />
          </button>
          <span className="toolbar-divider" />
          <button
            className="timeline-icon-button"
            disabled={!selectedClip || busy}
            onClick={() => void splitSelectedClip()}
            data-tooltip="Split Selected Clip at Playhead (Ctrl+K)"
            data-tooltip-pos="top"
            aria-label="Split clip"
          >
            <Icon name="split" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={!selectedClip || busy}
            onClick={() => void rippleDeleteSelectedClip()}
            data-tooltip="Ripple Delete Selected Clip (Shift+Del)"
            data-tooltip-pos="top"
            aria-label="Ripple delete"
          >
            <Icon name="ripple" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={!selectedClip || busy}
            onClick={() => void removeSelectedClip()}
            data-tooltip="Remove Selected Clip (Del)"
            data-tooltip-pos="top"
            aria-label="Remove clip"
          >
            <Icon name="remove" />
          </button>
          <span className="toolbar-divider" />
          <button
            className="timeline-icon-button"
            disabled={selectedClips.length < 2 || busy}
            onClick={() => void linkSelectedClips()}
            data-tooltip="Link Selected Clips (Ctrl+L)"
            data-tooltip-pos="top"
            aria-label="Link clips"
          >
            <Icon name="link" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={(!selectedClip && selectedClips.length === 0) || busy}
            onClick={() => void unlinkSelectedClips()}
            data-tooltip="Unlink Selected Clips (Ctrl+Shift+L)"
            data-tooltip-pos="top"
            aria-label="Unlink clips"
          >
            <Icon name="unlink" />
          </button>
          <span className="toolbar-divider" />
          <button
            className="timeline-icon-button"
            disabled={(!selectedClip && selectedClips.length === 0) || busy}
            onClick={() => {
              setCompoundClipNameDraft(`Compound Clip ${(project?.sequences?.length ?? 1) + 1}`);
              setCompoundClipModalOpen(true);
            }}
            data-tooltip="Create Compound Clip (Ctrl+G)"
            data-tooltip-pos="top"
            aria-label="Create Compound Clip"
          >
            <span style={{ fontWeight: 800, fontSize: "14px" }}>⧉</span>
          </button>
          <button
            className="timeline-icon-button"
            disabled={!selectedClip || busy}
            onClick={() => void decomposeSelectedCompoundClip()}
            data-tooltip="Decompose Compound Clip in Place (Ctrl+Shift+G)"
            data-tooltip-pos="top"
            aria-label="Decompose Compound Clip"
          >
            <span style={{ fontWeight: 800, fontSize: "14px" }}>⧈</span>
          </button>
          <span className="toolbar-divider" />
          <button
            className="timeline-icon-button"
            disabled={busy}
            onClick={() => void invokeMenuCommand("timeline.set_mark_in")}
            data-tooltip="Set In Point (I)"
            data-tooltip-pos="top"
            aria-label="Set In point"
          >
            <Icon name="markIn" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={busy}
            onClick={() => void invokeMenuCommand("timeline.set_mark_out")}
            data-tooltip="Set Out Point (O)"
            data-tooltip-pos="top"
            aria-label="Set Out point"
          >
            <Icon name="markOut" />
          </button>
          <button
            className="timeline-icon-button"
            disabled={project?.sequence.mark_in == null && project?.sequence.mark_out == null}
            onClick={() => void invokeMenuCommand("timeline.clear_marks")}
            data-tooltip="Clear In/Out Markers (Alt+X)"
            data-tooltip-pos="top"
            aria-label="Clear markers"
          >
            <Icon name="clearMarks" />
          </button>
          <span className="toolbar-divider" />
          <button
            className="timeline-icon-button"
            onClick={() => zoomTimeline(1 / 1.2)}
            data-tooltip="Zoom Out Timeline (Ctrl+-)"
            data-tooltip-pos="top"
            aria-label="Zoom out"
          >
            <Icon name="zoomOut" />
          </button>
          <button
            className="timeline-icon-button"
            onClick={() => zoomTimeline(1.2)}
            data-tooltip="Zoom In Timeline (Ctrl++)"
            data-tooltip-pos="top"
            aria-label="Zoom in"
          >
            <Icon name="zoomIn" />
          </button>
          <span className="toolbar-divider" />
          <button
            type="button"
            className="timeline-icon-button"
            onClick={() => void handleCreateTrackGroup()}
            data-tooltip="Create Track Folder Group"
            data-tooltip-pos="top"
            aria-label="Create Track Group"
          >
            <span style={{ fontWeight: 700, fontSize: "13px" }}>📁＋</span>
          </button>
        </div>
      </div>
    </>
  );
}
