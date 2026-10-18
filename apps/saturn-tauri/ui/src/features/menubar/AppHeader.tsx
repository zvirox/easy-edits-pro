import { useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "../../components/Icon";
import { ThemeToggle } from "../../components/interior/theme-toggle";
import { SegmentedControl } from "../../components/interior/segmented-control";
import type { JobInfo } from "../../app/useJobs";
import type { CommandStatus } from "../../app/ProjectProvider";
import type { MediaAsset, ProjectDocument, ProjectStatus } from "../../app/projectTypes";

export type DavinciWorkspace = "media" | "edit" | "fusion" | "color" | "audio" | "effects" | "keyframes" | "deliver";
export type Workspace = "edit" | "color" | "effects" | "audio" | "keyframes" | "export" | "library" | "ai";
export type SelectedClip = { track_id: number; clip_index: number };
export type CommandSpec = { id: string; label: string; menu: string[]; shortcut: string | null; mutating: boolean };

export const davinciWorkspaces: { id: DavinciWorkspace; label: string; shortcut: string; icon: string; isExperimental?: boolean }[] = [
  { id: "media", label: "Media", shortcut: "Shift+1", icon: "media" },
  { id: "edit", label: "Edit", shortcut: "Shift+2", icon: "edit" },
  { id: "fusion", label: "Fusion", shortcut: "Shift+3", icon: "effects", isExperimental: true },
  { id: "color", label: "Color", shortcut: "Shift+4", icon: "color" },
  { id: "audio", label: "Fairlight", shortcut: "Shift+5", icon: "audio" },
  { id: "effects", label: "Effects", shortcut: "Shift+6", icon: "effects" },
  { id: "keyframes", label: "Curves", shortcut: "Shift+7", icon: "keyframe" },
  { id: "deliver", label: "Deliver", shortcut: "Shift+8", icon: "deliver" },
];

export const workspaces: { id: Workspace; label: string }[] = [
  { id: "edit", label: "Edit" },
  { id: "color", label: "Color" },
  { id: "effects", label: "Effects" },
  { id: "audio", label: "Audio" },
  { id: "keyframes", label: "Keyframes" },
  { id: "export", label: "Export" },
  { id: "library", label: "Asset library" },
  { id: "ai", label: "AI assistant" },
];

export interface AppHeaderProps {
  project: ProjectDocument["project"] | null;
  projectState: ProjectStatus | null;
  commands: CommandSpec[];
  busy: boolean;
  undoStatus: CommandStatus;
  redoStatus: CommandStatus;
  selectedClip: SelectedClip | null;
  selectedMedia: MediaAsset | null;
  snapEnabled: boolean;
  workspace: Workspace;
  davinciWorkspace: DavinciWorkspace;
  selectDavinciWorkspace: (ws: DavinciWorkspace) => void;
  invokeMenuCommand: (id: string) => Promise<void>;
  ensureDiscardIsOkay: () => Promise<boolean>;
  setNewProjectName: (name: string) => void;
  setNewProjectOpen: (open: boolean) => void;
  openProject: () => Promise<void>;
  saveProject: () => Promise<void>;
  importMedia: () => Promise<void>;
  setNotice: (notice: string | null) => void;
  collabUser: { id: string; name: string; color: string };
  setCollabModalOpen: (open: boolean) => void;
  activeJobs: JobInfo[];
  jobs: JobInfo[];
  cancelJob: (id: number) => Promise<void>;
  cancelAllJobs: () => Promise<void>;
  clearJobHistory: () => Promise<void>;
  setBatchQueueDrawerOpen: (open: boolean) => void;
  setAiChatDrawerOpen: (open: boolean) => void;
  theme: "dark" | "light";
  toggleTheme: () => void;
  fullscreen: boolean;
  toggleFullscreen: () => void;
}

export function AppHeader({
  project,
  projectState,
  commands,
  busy,
  undoStatus,
  redoStatus,
  selectedClip,
  selectedMedia,
  snapEnabled,
  workspace,
  davinciWorkspace,
  selectDavinciWorkspace,
  invokeMenuCommand,
  ensureDiscardIsOkay,
  setNewProjectName,
  setNewProjectOpen,
  openProject,
  saveProject,
  importMedia,
  setNotice,
  collabUser,
  setCollabModalOpen,
  activeJobs,
  jobs,
  cancelJob,
  cancelAllJobs,
  clearJobHistory,
  setBatchQueueDrawerOpen,
  setAiChatDrawerOpen,
  theme,
  toggleTheme,
  fullscreen,
  toggleFullscreen,
}: AppHeaderProps) {
  const [jobDrawerOpen, setJobDrawerOpen] = useState(false);
  const jobDrawerRef = useRef<HTMLDivElement | null>(null);

  function renderMenu(menu: string) {
    const commandItems = commands
      .filter((command) => command.menu[0] === menu)
      .map((command) => ({
        id: command.id,
        label: command.label,
        shortcut: command.shortcut,
        disabled:
          busy ||
          (command.id === "edit.undo" && !undoStatus.enabled) ||
          (command.id === "edit.redo" && !redoStatus.enabled) ||
          (["timeline.split_clip", "timeline.ripple_delete", "timeline.remove_clip"].includes(command.id) &&
            !selectedClip) ||
          (command.id === "timeline.add_clip" && !selectedMedia),
      }));

    const extraItems: { id: string; label: string; shortcut?: string | null; disabled?: boolean }[] =
      menu === "File"
        ? [
            { id: "file.import_interop", label: "Import Timeline (EDL, FCPXML, Premiere)…", shortcut: null },
            { id: "file.export_interop", label: "Export Timeline (EDL, FCPXML, Premiere)…", shortcut: null },
            { id: "speech.transcribe", label: "Auto-Transcribe Speech (Whisper AI)…", shortcut: null },
            { id: "speech.silence_removal", label: "Remove Silence & Filler Words…", shortcut: null },
            { id: "app.export", label: "Export Media File…", shortcut: null },
            { id: "app.quit", label: "Quit", shortcut: "Ctrl+Q" },
          ]
        : menu === "View"
        ? [
            { id: "view.toggle_snap", label: `Snap to frame${snapEnabled ? " ✓" : ""}`, shortcut: "S" },
            { id: "view.zoom_in", label: "Zoom In", shortcut: "+" },
            { id: "view.zoom_out", label: "Zoom Out", shortcut: "−" },
            { id: "view.zoom_reset", label: "Reset Timeline Zoom", shortcut: "0" },
          ]
        : menu === "Window"
        ? workspaces.map((item) => ({
            id: `workspace.${item.id}`,
            label: `${item.label}${workspace === item.id ? " ✓" : ""}`,
            shortcut: null,
          }))
        : menu === "Help"
        ? [
            { id: "help.shortcuts", label: "Keyboard Shortcuts", shortcut: null },
            { id: "help.diagnostics", label: "Copy System Diagnostics", shortcut: null },
            { id: "help.support", label: "Report Issue / Support", shortcut: null },
            { id: "help.about", label: "About Easy Edits Pro", shortcut: null },
          ]
        : [];

    const items = [...commandItems, ...extraItems];

    return (
      <details className="app-menu" key={menu}>
        <summary>{menu}</summary>
        <div className="menu-popover">
          {items.map((item) => (
            <button
              key={item.id}
              onClick={(event) => {
                void invokeMenuCommand(item.id).catch((error: unknown) => setNotice(String(error)));
                event.currentTarget.closest("details")?.removeAttribute("open");
              }}
              disabled={item.disabled}
            >
              <span>{item.label}</span>
              {item.shortcut && <small>{item.shortcut.replace("Ctrl", "⌘/Ctrl")}</small>}
            </button>
          ))}
        </div>
      </details>
    );
  }

  return (
    <header className="app-header">
      {/* Tier 1 (32px System Menu & Global Utilities) */}
      <div className="system-bar-tier1">
        <div className="system-bar-left">
          <div className="brand">
            <img className="brand-mark" src="/saturn-camera.png" alt="Easy Edits Pro camera logo" />
            <span className="brand-title">Easy Edits Pro</span>
          </div>
          <div className="project-badge" title="Active Project">
            <span>{project?.name ?? "Untitled project"}</span>
            {projectState?.is_dirty && <span className="dirty-indicator" title="Unsaved changes">•</span>}
          </div>
          <nav className="menu-bar" aria-label="Application menus">
            {["File", "Edit", "Clip", "Sequence", "Markers", "View", "Window", "Help"].map(renderMenu)}
          </nav>
          <nav className="file-actions" aria-label="Project actions" style={{ display: "flex", gap: "2px" }}>
            <button
              className="icon-button"
              onClick={() => {
                void ensureDiscardIsOkay().then((ok) => {
                  if (ok) {
                    setNewProjectName("Untitled project");
                    setNewProjectOpen(true);
                  }
                });
              }}
              data-tooltip="New Project (Ctrl+N)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="fileNew" />
            </button>
            <button
              className="icon-button"
              onClick={() => void openProject().catch((error: unknown) => setNotice(String(error)))}
              data-tooltip="Open Project (Ctrl+O)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="fileOpen" />
            </button>
            <button
              className="icon-button"
              onClick={() => void saveProject()}
              data-tooltip="Save Project (Ctrl+S)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="fileSave" />
            </button>
            <button
              className="icon-button"
              onClick={() => void importMedia()}
              disabled={busy}
              data-tooltip="Import Media Files (Ctrl+I)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="import" />
            </button>
          </nav>
        </div>

        <div className="system-bar-right">
          <button
            className="collab-pill-btn"
            onClick={() => setCollabModalOpen(true)}
            data-tooltip="Collaborative Multi-User Project Locking (Phase 12.1)"
            data-tooltip-pos="bottom"
          >
            <span className="collab-indicator-dot" style={{ backgroundColor: collabUser.color }} />
            <span className="collab-name">{collabUser.name}</span>
            <span className="collab-badge">
              🔒 {project?.collaboration?.locks?.length ?? 0}
            </span>
          </button>

          {/* Background Job Queue Status Pill */}
          <div className="job-queue-wrapper" ref={jobDrawerRef}>
            <button
              className={`job-status-pill ${activeJobs.length > 0 ? "has-active" : ""}`}
              onClick={() => setJobDrawerOpen((prev) => !prev)}
              aria-label="Background Task Queue"
              data-tooltip={
                activeJobs.length > 0
                  ? `${activeJobs.length} active background task${activeJobs.length === 1 ? "" : "s"}`
                  : "Background Tasks (All idle)"
              }
              data-tooltip-pos="bottom"
            >
              {activeJobs.length > 0 ? (
                <>
                  <span className="job-pill-spinner" />
                  <span className="job-pill-count">{activeJobs.length}</span>
                  <span className="job-pill-label">
                    {activeJobs[0]?.state.status === "running"
                      ? `${Math.round((activeJobs[0].state.progress || 0) * 100)}%`
                      : "Queued"}
                  </span>
                </>
              ) : (
                <>
                  <Icon name="check" />
                  <span className="job-pill-idle">Tasks</span>
                </>
              )}
            </button>

            {jobDrawerOpen && (
              <div className="job-drawer-popover">
                <header className="job-drawer-header">
                  <div>
                    <strong>Background Tasks</strong>
                    <small>
                      {activeJobs.length} active · {jobs.length} total
                    </small>
                  </div>
                  <div className="job-drawer-actions">
                    {activeJobs.length > 0 && (
                      <button
                        className="job-cancel-all-btn"
                        onClick={() => void cancelAllJobs()}
                        title="Cancel all running tasks"
                      >
                        Cancel All
                      </button>
                    )}
                    <button
                      className="job-clear-btn"
                      onClick={() => void clearJobHistory()}
                      title="Clear completed history"
                    >
                      Clear History
                    </button>
                  </div>
                </header>

                <div className="job-drawer-list scrollbar-thin">
                  {jobs.length === 0 ? (
                    <div className="job-drawer-empty">No background tasks</div>
                  ) : (
                    jobs.map((job) => (
                      <div key={job.id} className={`job-item job-${job.state.status}`}>
                        <div className="job-item-header">
                          <span className="job-item-label" title={job.label}>
                            {job.label}
                          </span>
                          <span className={`job-item-status-badge badge-${job.state.status}`}>
                            {job.state.status}
                          </span>
                          {(job.state.status === "running" || job.state.status === "queued") && (
                            <button
                              className="job-item-cancel"
                              onClick={() => void cancelJob(job.id)}
                              title="Cancel this task"
                            >
                              ✕
                            </button>
                          )}
                        </div>

                        {job.state.status === "running" && (
                          <div className="job-progress-bar-wrapper">
                            <div
                              className="job-progress-bar-fill"
                              style={{ width: `${Math.round((job.state.progress || 0) * 100)}%` }}
                            />
                            <div className="job-progress-text-row">
                              <small>{job.state.phase || "Processing…"}</small>
                              <small>{Math.round((job.state.progress || 0) * 100)}%</small>
                            </div>
                          </div>
                        )}

                        {job.state.status === "failed" && (
                          <div className="job-error-text">{job.state.error}</div>
                        )}
                      </div>
                    ))
                  )}
                </div>
              </div>
            )}
          </div>

          <button
            type="button"
            className="system-util-btn"
            onClick={() => setBatchQueueDrawerOpen(true)}
            data-tooltip="Open Background Batch Render Queue"
            data-tooltip-pos="bottom"
          >
            <span>📦</span>
            <span>Queue</span>
          </button>

          <button
            type="button"
            className="system-util-btn ai-btn"
            onClick={() => setAiChatDrawerOpen(true)}
            data-tooltip="Open Local AI Assistant (Ollama) • Experimental"
            data-tooltip-pos="bottom"
          >
            <span>🧠</span>
            <span>AI Assistant</span>
            <span className="exp-badge">EXP</span>
          </button>

          <ThemeToggle theme={theme} onToggle={toggleTheme} />

          <div className="window-controls">
            <button
              className="icon-button"
              onClick={() => void toggleFullscreen()}
              aria-label={fullscreen ? "Leave fullscreen" : "Enter fullscreen"}
              data-tooltip={fullscreen ? "Leave Fullscreen (F11)" : "Enter Fullscreen (F11)"}
              data-tooltip-pos="bottom"
            >
              <Icon name={fullscreen ? "restore" : "fullscreen"} />
            </button>
            <button
              className="icon-button close"
              onClick={() => void getCurrentWindow().close()}
              aria-label="Close Easy Edits Pro"
              data-tooltip="Close Easy Edits Pro (Ctrl+Q)"
              data-tooltip-pos="bottom"
            >
              <Icon name="close" />
            </button>
          </div>
        </div>
      </div>

      {/* Tier 2 (36px Dedicated Workspace Bar) */}
      <div className="workspace-bar-tier2">
        <SegmentedControl
          label="Workspaces"
          options={davinciWorkspaces.map((w) => ({ value: w.id, label: w.label }))}
          value={davinciWorkspace}
          onValueChange={(val) => selectDavinciWorkspace(val as DavinciWorkspace)}
        />
      </div>
    </header>
  );
}
