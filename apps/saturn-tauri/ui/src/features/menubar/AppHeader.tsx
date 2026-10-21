import { InteriorButton } from "../../components/interior/interior-button";
import { useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { backdropVariants, dialogVariants, menuVariants, useMotionPreset } from "../../motion/variants";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "../../components/Icon";
import { ThemeToggle } from "../../components/interior/theme-toggle";
import { SegmentedControl } from "../../components/interior/segmented-control";
import type { JobInfo } from "../../app/useJobs";
import type { CommandStatus } from "../../app/ProjectProvider";
import { formatShortcut } from "../shortcuts/shortcuts.ts";
import { rankCommands } from "./ranking.ts";
import type {
  DavinciWorkspace,
  MediaAsset,
  ProjectDocument,
  ProjectStatus,
  SelectedClip,
} from "../../app/projectTypes";

export type { DavinciWorkspace, SelectedClip };
export type Workspace = "edit" | "color" | "effects" | "audio" | "keyframes" | "export" | "library" | "ai";
export type CommandSpec = {
  id: string;
  label: string;
  menu: string[];
  shortcut: string | null;
  params: string;
  category: string;
  mutating: boolean;
  undoGroup: "none" | "transaction" | "gesture";
  aliases: string[];
  since: string;
  shipped: boolean;
  uiOnly: boolean;
};

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
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [commandQuery, setCommandQuery] = useState("");
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const menuBarRef = useRef<HTMLElement | null>(null);
  const jobDrawerRef = useRef<HTMLDivElement | null>(null);
  const commandSearchRef = useRef<HTMLInputElement | null>(null);
  const motionPreset = useMotionPreset();
  const matchingCommands = useMemo(
    () => rankCommands(commands, commandQuery),
    [commandQuery, commands],
  );

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandPaletteOpen((open) => !open);
        setCommandQuery("");
      } else if (event.key === "Escape") {
        setCommandPaletteOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (commandPaletteOpen) commandSearchRef.current?.focus();
  }, [commandPaletteOpen]);

  useEffect(() => {
    if (openMenu === null) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenMenu(null);
    };
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (menuBarRef.current && !menuBarRef.current.contains(event.target as Node)) setOpenMenu(null);
    };
    window.addEventListener("keydown", closeOnEscape);
    window.addEventListener("pointerdown", closeOnOutsidePointer);
    return () => {
      window.removeEventListener("keydown", closeOnEscape);
      window.removeEventListener("pointerdown", closeOnOutsidePointer);
    };
  }, [openMenu]);

  function disabledReason(id: string): string | null {
    if (busy) return "Working…";
    if (id === "edit.undo") return undoStatus.enabled ? null : undoStatus.reason ?? "Nothing to undo";
    if (id === "edit.redo") return redoStatus.enabled ? null : redoStatus.reason ?? "Nothing to redo";
    if (
      [
        "timeline.split_clip",
        "timeline.ripple_delete",
        "timeline.remove_clip",
        "timeline.rate_stretch",
      ].includes(id) &&
      !selectedClip
    ) {
      return "Select a timeline clip first";
    }
    if (id === "timeline.add_clip" && !selectedMedia) return "Select a media item first";
    return null;
  }

  function renderMenu(menu: string) {
    // Preserve catalogue order, grouped by the second menu segment.
    const groups = new Map<string, { id: string; label: string; shortcut: string | null; reason: string | null }[]>();
    for (const command of commands) {
      if (!command.shipped || command.menu[0] !== menu) continue;
      const section = command.menu.length > 1 ? command.menu[1] : "";
      const activeWorkspace =
        command.id.startsWith("workspace.") &&
        (command.id.slice("workspace.".length) === davinciWorkspace ||
          command.id.slice("workspace.".length) === workspace);
      const entry = {
        id: command.id,
        label:
          (command.id === "view.toggle_snap" && snapEnabled) || activeWorkspace
            ? `${command.label} ✓`
            : command.label,
        shortcut: command.shortcut,
        reason: disabledReason(command.id),
      };
      const bucket = groups.get(section);
      if (bucket) bucket.push(entry);
      else groups.set(section, [entry]);
    }

    return (
      <div className="app-menu" key={menu}>
        <button
          type="button"
          className="app-menu-trigger"
          aria-haspopup="menu"
          aria-expanded={openMenu === menu}
          onClick={() => setOpenMenu((current) => (current === menu ? null : menu))}
        >
          {menu}
        </button>
        <AnimatePresence>
          {openMenu === menu && (
            <motion.div
              className="menu-popover glass"
              data-elev="3"
              role="menu"
              initial={motionPreset.initial}
              animate="animate"
              exit="exit"
              variants={menuVariants}
              transition={motionPreset.transition}
              style={{ transformOrigin: "top left" }}
            >
              {groups.size === 0 && <div className="menu-section-label">No commands yet</div>}
              {[...groups.entries()].map(([section, items]) => (
                <div className="menu-section" key={section || "root"}>
                  {section && <div className="menu-section-label">{section}</div>}
                  {items.map((item) => (
                    <InteriorButton
                      key={item.id}
                      title={item.reason ?? undefined}
                      disabled={item.reason !== null}
                      onClick={(event) => {
                        setOpenMenu(null);
                        event.currentTarget.blur();
                        void invokeMenuCommand(item.id).catch((error: unknown) =>
                          setNotice(String(error)),
                        );
                      }}
                    >
                      <span>{item.label}</span>
                      {item.shortcut && <small>{formatShortcut(item.shortcut)}</small>}
                    </InteriorButton>
                  ))}
                </div>
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  }

  return (
    <>
    <header className="app-header glass" data-elev="1">
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
          <InteriorButton
            className="command-search-trigger"
            onClick={() => { setCommandQuery(""); setCommandPaletteOpen(true); }}
            aria-label="Search editor commands"
            title="Search commands (Ctrl+K)"
          >
            <Icon name="search" />
            <span>Find command</span>
            <kbd>Ctrl K</kbd>
          </InteriorButton>
          <nav className="menu-bar" aria-label="Application menus" ref={menuBarRef}>
            {["File", "Edit", "Clip", "Sequence", "Markers", "Graphics", "Audio", "Effects", "View", "Window", "Help"]
              .filter((menu) =>
                commands.some((command) => command.shipped && command.menu[0] === menu)
              )
              .map(renderMenu)}
          </nav>
          <nav className="file-actions" aria-label="Project actions" style={{ display: "flex", gap: "2px" }}>
            <InteriorButton
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
            </InteriorButton>
            <InteriorButton
              className="icon-button"
              onClick={() => void openProject().catch((error: unknown) => setNotice(String(error)))}
              data-tooltip="Open Project (Ctrl+O)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="fileOpen" />
            </InteriorButton>
            <InteriorButton
              className="icon-button"
              onClick={() => void saveProject()}
              data-tooltip="Save Project (Ctrl+S)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="fileSave" />
            </InteriorButton>
            <InteriorButton
              className="icon-button"
              onClick={() => void importMedia()}
              disabled={busy}
              data-tooltip="Import Media Files (Ctrl+I)"
              data-tooltip-pos="bottom"
              style={{ width: "24px", height: "24px", padding: "3px" }}
            >
              <Icon name="import" />
            </InteriorButton>
          </nav>
        </div>

        <div className="system-bar-right">
          <InteriorButton
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
          </InteriorButton>

          {/* Background Job Queue Status Pill */}
          <div className="job-queue-wrapper" ref={jobDrawerRef}>
            <InteriorButton
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
            </InteriorButton>

            <AnimatePresence>
              {jobDrawerOpen && (
                <motion.div
                  className="job-drawer-popover glass"
                  data-elev="3"
                  initial={motionPreset.initial}
                  animate="animate"
                  exit="exit"
                  variants={menuVariants}
                  transition={motionPreset.transition}
                  style={{ transformOrigin: "top right" }}
                >
                <header className="job-drawer-header">
                  <div>
                    <strong>Background Tasks</strong>
                    <small>
                      {activeJobs.length} active · {jobs.length} total
                    </small>
                  </div>
                  <div className="job-drawer-actions">
                    {activeJobs.length > 0 && (
                      <InteriorButton
                        className="job-cancel-all-btn"
                        onClick={() => void cancelAllJobs()}
                        title="Cancel all running tasks"
                      >
                        Cancel All
                      </InteriorButton>
                    )}
                    <InteriorButton
                      className="job-clear-btn"
                      onClick={() => void clearJobHistory()}
                      title="Clear completed history"
                    >
                      Clear History
                    </InteriorButton>
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
                            <InteriorButton
                              className="job-item-cancel"
                              onClick={() => void cancelJob(job.id)}
                              title="Cancel this task"
                            >
                              ✕
                            </InteriorButton>
                          )}
                        </div>

                        {job.state.status === "running" && (
                          <div className="job-progress-bar-wrapper">
                            <div
                              className="job-progress-bar-fill"
                              style={{ transform: `scaleX(${Math.max(0, Math.min(1, job.state.progress || 0))})` }}
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
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          <InteriorButton
            type="button"
            className="system-util-btn"
            onClick={() => setBatchQueueDrawerOpen(true)}
            data-tooltip="Open Background Batch Render Queue"
            data-tooltip-pos="bottom"
          >
            <span>📦</span>
            <span>Queue</span>
          </InteriorButton>

          <InteriorButton
            type="button"
            className="system-util-btn ai-btn"
            onClick={() => setAiChatDrawerOpen(true)}
            data-tooltip="Open Local AI Assistant (Ollama) • Experimental"
            data-tooltip-pos="bottom"
          >
            <span>🧠</span>
            <span>AI Assistant</span>
            <span className="exp-badge">EXP</span>
          </InteriorButton>

          <ThemeToggle theme={theme} onToggle={toggleTheme} />

          <div className="window-controls">
            <InteriorButton
              className="icon-button"
              onClick={() => void toggleFullscreen()}
              aria-label={fullscreen ? "Leave fullscreen" : "Enter fullscreen"}
              data-tooltip={fullscreen ? "Leave Fullscreen (F11)" : "Enter Fullscreen (F11)"}
              data-tooltip-pos="bottom"
            >
              <Icon name={fullscreen ? "restore" : "fullscreen"} />
            </InteriorButton>
            <InteriorButton
              className="icon-button close"
              onClick={() => void getCurrentWindow().close()}
              aria-label="Close Easy Edits Pro"
              data-tooltip="Close Easy Edits Pro (Ctrl+Q)"
              data-tooltip-pos="bottom"
            >
              <Icon name="close" />
            </InteriorButton>
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
    <AnimatePresence>
    {commandPaletteOpen && (
      <motion.div
        className="command-palette-backdrop"
        onMouseDown={(event) => { if (event.target === event.currentTarget) setCommandPaletteOpen(false); }}
        initial={motionPreset.initial}
        animate="animate"
        exit="exit"
        variants={backdropVariants}
        transition={motionPreset.transition}
      >
        <motion.section
          className="command-palette glass"
          data-elev="4"
          role="dialog"
          aria-modal="true"
          aria-labelledby="command-palette-title"
          initial={motionPreset.initial}
          animate="animate"
          exit="exit"
          variants={dialogVariants}
          transition={motionPreset.transition}
          style={{ transformOrigin: "top center" }}
        >
          <label id="command-palette-title" className="sr-only" htmlFor="command-palette-search">Find an editor command</label>
          <div className="command-palette-search">
            <Icon name="search" />
            <input
              id="command-palette-search"
              ref={commandSearchRef}
              autoFocus
              value={commandQuery}
              onChange={(event) => setCommandQuery(event.target.value)}
              placeholder="Find tools and actions…"
            />
            <kbd>ESC</kbd>
          </div>
          <div className="command-palette-results" role="listbox" aria-label="Commands">
            {matchingCommands.length ? matchingCommands.map((command) => (
              <InteriorButton
                key={command.id}
                className="command-palette-result"
                role="option"
                aria-selected="false"
                onClick={() => {
                  setCommandPaletteOpen(false);
                  void invokeMenuCommand(command.id).catch((error: unknown) => setNotice(String(error)));
                }}
              >
                <span className="command-palette-result-label">{command.label}</span>
                <span className="command-palette-result-meta">{command.menu.join(" › ") || command.id}</span>
                {command.shortcut && <kbd>{formatShortcut(command.shortcut)}</kbd>}
              </InteriorButton>
            )) : <p className="command-palette-empty">No matching commands.</p>}
          </div>
          <footer><span>Search by tool name or command id</span><span><kbd>Ctrl K</kbd> to toggle</span></footer>
        </motion.section>
      </motion.div>
    )}
    </AnimatePresence>
    </>
  );
}
