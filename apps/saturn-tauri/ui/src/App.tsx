import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProjectProvider, runCommand } from "./app/ProjectProvider";
import { useProject } from "./app/useProject";
import { usePlayback } from "./features/preview";
import { useJobs } from "./app/useJobs";
import { useRecovery, ProjectWelcomeScreen } from "./features/welcome";
import { useMediaServer } from "./features/media-pool";
import { useShortcuts, type DavinciWorkspace } from "./app/useShortcuts";
import { useMediaOperations } from "./app/useMediaOperations";
import { useTimelineOperations } from "./app/useTimelineOperations";
import { useInspectorOperations } from "./app/useInspectorOperations";
import { useProjectOperations } from "./app/useProjectOperations";
import { useAudioWorkspace } from "./app/useAudioWorkspace";

import { AppHeader } from "./features/menubar";
import { WorkspaceRouter } from "./workspaces/WorkspaceRouter";
import { StatusBar } from "./components/StatusBar";
import { EditorModals } from "./features/dialogs";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { TICKS_PER_SECOND, timelineTools } from "./features/timeline/types";
import { audioClockMaster } from "./engine/playbackEngine";
import { ShortcutsDialog } from "./features/shortcuts/ShortcutsDialog";
import { PreferencesDialog } from "./features/preferences/PreferencesDialog";
import {
  DEFAULT_PREFERENCES,
  loadPreferences,
  savePreferences,
  type Preferences,
} from "./features/preferences/preferences";
import { setReducedMotionOverride } from "./motion/variants";
import { formatTrimStatus } from "./app/timelineTrim";
import {
  PANEL_LAYOUT_EVENT,
  PANEL_LAYOUT_STORAGE_PREFIX,
  PANEL_RESET_EVENT,
  PANEL_TOGGLE_EVENT,
  workspacePresets,
} from "./features/panels/presets";
import {
  captureWorkspaceLayout,
  createCustomPreset,
  loadCustomPresets,
  saveCustomPresets,
  type CustomWorkspacePreset,
} from "./features/panels/presetStore";
import {
  LayoutPresetsDialog,
  type AnyWorkspacePreset,
} from "./features/panels/LayoutPresetsDialog";
import type { CommandSpec, SelectedClip } from "./types";

function EditorApp() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const theme = preferences.theme;
  const setTheme = useCallback((next: Preferences["theme"]) => {
    setPreferences((current) => ({ ...current, theme: next }));
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.documentElement.classList.toggle("light", theme === "light");
    try {
      localStorage.setItem("eep-theme", theme);
    } catch {}
  }, [theme]);

  useEffect(() => {
    savePreferences(preferences);
  }, [preferences]);

  useEffect(() => {
    setReducedMotionOverride(preferences.reducedMotion);
  }, [preferences.reducedMotion]);

  // The runtime probe owns the first glass tag; only apply explicit changes so
  // a slow host's lite fallback is not overridden on startup.
  const glassApplied = useRef(false);
  useEffect(() => {
    if (!glassApplied.current) {
      glassApplied.current = true;
      return;
    }
    document.documentElement.setAttribute("data-glass", preferences.glass);
  }, [preferences.glass]);

  const toggleTheme = useCallback(
    () => setTheme(theme === "dark" ? "light" : "dark"),
    [setTheme, theme]
  );
  const [commands, setCommands] = useState<CommandSpec[]>([]);
  useEffect(() => {
    invoke<CommandSpec[]>("get_command_catalogue")
      .then((catalogue) => setCommands(catalogue))
      .catch(() => setCommands([]));
  }, []);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [presetsOpen, setPresetsOpen] = useState(false);
  const [customPresets, setCustomPresets] = useState<CustomWorkspacePreset[]>(loadCustomPresets);
  const [shortcutOverrides, setShortcutOverrides] = useState<Record<string, string>>(() => {
    try {
      const raw = localStorage.getItem("easy-edits-pro.shortcuts.v1");
      const parsed = raw ? JSON.parse(raw) : null;
      return parsed && typeof parsed === "object" ? (parsed as Record<string, string>) : {};
    } catch {
      return {};
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem("easy-edits-pro.shortcuts.v1", JSON.stringify(shortcutOverrides));
    } catch {}
  }, [shortcutOverrides]);
  const effectiveCommands = useMemo(
    () =>
      commands.map((command) => {
        const override = shortcutOverrides[command.id];
        return override === undefined
          ? command
          : { ...command, shortcut: override || null };
      }),
    [commands, shortcutOverrides]
  );

  const {
    projectState, setProjectState, undoStatus, redoStatus,
    setUndoStatus, setRedoStatus, missingMediaIds, setMissingMediaIds,
    connection, refreshProject,
  } = useProject();

  const project = projectState?.document.project ?? null;
  const media = project?.media ?? [];
  const recoveryStatus = useRecovery();
  const jobsHook = useJobs();
  const { mediaServerPort, mediaServerToken, getMediaUrl } = useMediaServer();

  const [programPreviewError, setProgramPreviewError] = useState<string | null>(null);
  const [preparedPreviews] = useState<Record<number, string>>({});

  const playback = usePlayback({
    projectState, project, media, preparedPreviews,
    mediaServerPort, mediaServerToken,
    clearProgramPreviewError: () => setProgramPreviewError(null),
  });

  const perform = useCallback(async (id: string, params: unknown = null, successNotice?: string) => {
    setBusy(true);
    setNotice("");
    try {
      await runCommand(id, params);
      await refreshProject();
      if (successNotice) setNotice(successNotice);
      return true;
    } catch (error) {
      setNotice(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }, [refreshProject]);

  const menuContext = useRef<{ selectedClip: SelectedClip | null; playhead: number }>({
    selectedClip: null,
    playhead: 0,
  });

  const projectOps = useProjectOperations({
    projectState, setProjectState, refreshProject,
    perform, runCommand, setNotice, setBusy, busy,
    setPlayhead: playback.setPlayhead,
    setSelectedClip: () => {}, setSelectedClips: () => {},
    setSelectedTransition: () => {}, setSelectedMediaId: () => {},
    getMenuContext: () => menuContext.current,
  });

  const timelineEndSeconds = Math.max(
    30,
    ...media.map((item) => (item.duration ?? 0) / TICKS_PER_SECOND),
    ...(project?.sequence.tracks ?? []).flatMap((track) =>
      track.clips.map((clip) => (clip.timeline_start + clip.duration) / TICKS_PER_SECOND)
    )
  );

  const timelineOps = useTimelineOperations({
    project, media, playhead: playback.playhead,
    seekProgramTo: playback.seekProgramTo, timelineEndSeconds,
    perform, runCommand, setNotice, busy, refreshProject,
    setCompoundClipModalOpen: projectOps.setCompoundClipModalOpen,
    setCompoundClipNameDraft: projectOps.setCompoundClipNameDraft,
    keyframeProperty: "opacity", keyframeValue: "1",
    initialSnapEnabled: preferences.snap,
    initialTrackHeight: preferences.trackHeight,
  });

  useEffect(() => {
    setPreferences((current) =>
      current.snap === timelineOps.snapEnabled
        ? current
        : { ...current, snap: timelineOps.snapEnabled }
    );
  }, [timelineOps.snapEnabled]);

  useEffect(() => {
    setPreferences((current) =>
      current.trackHeight === timelineOps.timelineTrackHeight
        ? current
        : { ...current, trackHeight: timelineOps.timelineTrackHeight }
    );
  }, [timelineOps.timelineTrackHeight]);

  useEffect(() => {
    menuContext.current = {
      selectedClip: timelineOps.selectedClip,
      playhead: playback.playhead,
    };
  }, [timelineOps.selectedClip, playback.playhead]);

  const audioOps = useAudioWorkspace(project, perform, playback.programPlaying);

  const activeTool = timelineTools.find((tool) => tool.id === timelineOps.timelineTool);

  const selectedTimelineClip =
    timelineOps.selectedClip && project
      ? project.sequence.tracks.find((t) => t.id === timelineOps.selectedClip!.track_id)?.clips[
          timelineOps.selectedClip.clip_index
        ] ?? null
      : null;

  const mediaOps = useMediaOperations({
    project, setProjectState, setMissingMediaIds, setUndoStatus, setRedoStatus,
    playhead: playback.playhead, setPlayhead: playback.setPlayhead, refreshProject,
    perform, runCommand, setNotice, setBusy, busy,
    selectedClip: timelineOps.selectedClip, setSelectedClip: timelineOps.setSelectedClip,
    setSelectedClips: timelineOps.setSelectedClips, selectedTransition: timelineOps.selectedTransition,
    setSelectedTransition: timelineOps.setSelectedTransition, selectedTimelineClip,
    setWorkspace: projectOps.setWorkspace,
    setMediaDropTargetTrackId: timelineOps.setMediaDropTargetTrackId,
    timelineZoom: timelineOps.timelineZoom,
    snapEnabled: timelineOps.snapEnabled,
  });

  const inspectorOps = useInspectorOperations({
    selectedClip: timelineOps.selectedClip, selectedTimelineClip,
    playhead: playback.playhead, perform, runCommand, refreshProject,
    setNotice, setBusy, project,
  });

  const selectedMedia = media.find((item) => item.id === mediaOps.selectedMediaId) ?? null;

  const selectDavinciWorkspace = useCallback((ws: DavinciWorkspace) => {
    projectOps.setDavinciWorkspace(ws);
    if (ws === "edit") projectOps.setWorkspace("edit");
    else if (ws === "color") projectOps.setWorkspace("color");
    else if (ws === "effects") projectOps.setWorkspace("effects");
    else if (ws === "audio") projectOps.setWorkspace("audio");
    else if (ws === "keyframes") projectOps.setWorkspace("keyframes");
  }, [projectOps]);

  const applyWorkspacePreset = useCallback(
    (preset: AnyWorkspacePreset) => {
      for (const [workspace, layout] of Object.entries(preset.layouts)) {
        const payload = { rects: layout.rects, hidden: layout.hidden };
        try {
          localStorage.setItem(
            `${PANEL_LAYOUT_STORAGE_PREFIX}.${workspace}`,
            JSON.stringify(payload)
          );
        } catch {}
        void invoke("save_panel_layout", { workspace, layout: payload }).catch(() => {});
      }
      window.dispatchEvent(new Event(PANEL_LAYOUT_EVENT));
      selectDavinciWorkspace(preset.workspace as DavinciWorkspace);
      setNotice(`Layout preset · ${preset.label}`);
      setPresetsOpen(false);
    },
    [selectDavinciWorkspace]
  );

  /**
   * Single entry point for menus, the command palette, keyboard shortcuts, and
   * workspace toolbars. UI-only commands are handled here; everything else goes
   * through the engine-aware `invokeMenuCommand`.
   */
  const dispatchCommand = useCallback(
    async (id: string) => {
      if (id.startsWith("workspace.preset_")) {
        const preset = workspacePresets.find(
          (entry) => `workspace.preset_${entry.id}` === id
        );
        if (!preset) return;
        applyWorkspacePreset(preset);
        return;
      }
      if (id === "workspace.manage_presets") {
        setPresetsOpen(true);
        return;
      }
      if (id === "window.reset_layout") {
        setPresetsOpen(false);
        window.dispatchEvent(new Event(PANEL_RESET_EVENT));
        setNotice("Panel layout reset");
        return;
      }
      if (id.startsWith("workspace.")) {
        selectDavinciWorkspace(id.slice("workspace.".length) as DavinciWorkspace);
        return;
      }
      switch (id) {
        case "project.new":
          projectOps.setNewProjectOpen(true);
          return;
        case "project.add_media":
          await mediaOps.importMedia();
          return;
        case "view.toggle_snap":
          timelineOps.setSnapEnabled((value) => !value);
          return;
        case "view.zoom_in":
          timelineOps.zoomTimeline(1.25);
          return;
        case "view.zoom_out":
          timelineOps.zoomTimeline(0.8);
          return;
        case "view.zoom_reset":
          timelineOps.setTimelineZoom(72);
          return;
        case "view.theme_dark":
        case "view.theme_light":
          setPreferences((current) => ({
            ...current,
            theme: id === "view.theme_dark" ? "dark" : "light",
          }));
          return;
        case "view.reduced_motion":
          setPreferences((current) => ({
            ...current,
            reducedMotion:
              current.reducedMotion === null ? true : current.reducedMotion ? false : null,
          }));
          return;
        case "view.panel_media":
        case "view.panel_inspector":
          window.dispatchEvent(
            new CustomEvent(PANEL_TOGGLE_EVENT, {
              detail: { id: id === "view.panel_media" ? "media" : "inspector" },
            }),
          );
          return;
        case "edit.preferences":
          setPreferencesOpen(true);
          return;
        case "help.shortcuts":
          setShortcutsOpen(true);
          return;
        case "help.diagnostics":
          setNotice("Diagnostics are available from the status bar.");
          return;
        case "help.support":
          setNotice("Report issues from the project repository.");
          return;
        case "help.about":
          setNotice("Easy Edits Pro — glass NLE built on the Saturn engine.");
          return;
      }
      if (id.startsWith("view.glass_")) {
        const glass = id.slice("view.glass_".length) as Preferences["glass"];
        setPreferences((current) => ({ ...current, glass }));
        return;
      }
      await projectOps.invokeMenuCommand(id);
    },
    [selectDavinciWorkspace, applyWorkspacePreset, projectOps, mediaOps, timelineOps]
  );

  const shortcuts = useShortcuts({
    commands: effectiveCommands.filter((command) => command.shipped),
    timelineTools: timelineTools.map((t) => ({ id: t.id, shortcut: t.shortcut })),
    project,
    multicamMode: false,
    selectedTransition: timelineOps.selectedTransition,
    selectedClip: timelineOps.selectedClip,
    timelineZoom: timelineOps.timelineZoom,
    toggleProgramPlayback: playback.toggleProgramPlayback,
    switchMulticamAngle: () => {},
    handleShuttleKey: playback.handleShuttleKey,
    stepProgramFrame: playback.stepProgramFrame,
    seekProgramTo: playback.seekProgramTo,
    removeSelectedTransition: mediaOps.removeSelectedTransition,
    removeSelectedClip: timelineOps.removeSelectedClip,
    addTitleToTimeline: mediaOps.addTitleToTimeline,
    addTransitionToTimeline: mediaOps.addTransitionToTimeline,
    unlinkSelectedClips: timelineOps.unlinkSelectedClips,
    linkSelectedClips: timelineOps.linkSelectedClips,
    decomposeSelectedCompoundClip: timelineOps.decomposeSelectedCompoundClip,
    setCompoundClipNameDraft: projectOps.setCompoundClipNameDraft,
    setCompoundClipModalOpen: projectOps.setCompoundClipModalOpen,
    toggleTheme,
    selectDavinciWorkspace,
    setTimelineTool: timelineOps.setTimelineTool,
    setSnapEnabled: timelineOps.setSnapEnabled,
    zoomTimeline: timelineOps.zoomTimeline,
    invokeMenuCommand: dispatchCommand,
    setNotice,
  });

  const saveCurrentPreset = useCallback(
    (label: string) => {
      const target = captureWorkspaceLayout(projectOps.workspace)
        ? projectOps.workspace
        : "edit";
      const layout = captureWorkspaceLayout(target);
      if (!layout) {
        setNotice("Arrange the floating panels first, then save the layout.");
        return;
      }
      const preset = createCustomPreset(label, target, layout, customPresets);
      const next = [...customPresets, preset];
      setCustomPresets(next);
      saveCustomPresets(next);
      setNotice(`Saved layout · ${preset.label}`);
    },
    [projectOps.workspace, customPresets]
  );

  const renameCustomPreset = useCallback(
    (id: string, label: string) => {
      const next = customPresets.map((preset) =>
        preset.id === id ? { ...preset, label } : preset
      );
      setCustomPresets(next);
      saveCustomPresets(next);
    },
    [customPresets]
  );

  const deleteCustomPreset = useCallback(
    (id: string) => {
      const next = customPresets.filter((preset) => preset.id !== id);
      setCustomPresets(next);
      saveCustomPresets(next);
      setNotice("Layout preset deleted");
    },
    [customPresets]
  );

  if (projectOps.appStage === "welcome") {
    return (
      <ProjectWelcomeScreen
        appStage={projectOps.appStage}
        loadingMessage={projectOps.loadingMessage}
        connection="Ready"
        welcomeMode={projectOps.welcomeMode} setWelcomeMode={projectOps.setWelcomeMode}
        setupProjectName={projectOps.setupProjectName} setSetupProjectName={projectOps.setSetupProjectName}
        setupSequenceName={projectOps.setupSequenceName} setSetupSequenceName={projectOps.setSetupSequenceName}
        setupResolution={projectOps.setupResolution} setSetupResolution={projectOps.setSetupResolution}
        setupWidth={projectOps.setupWidth} setSetupWidth={projectOps.setSetupWidth}
        setupHeight={projectOps.setupHeight} setSetupHeight={projectOps.setSetupHeight}
        setupFrameRate={projectOps.setupFrameRate} setSetupFrameRate={projectOps.setSetupFrameRate}
        recentProjects={projectOps.recentProjects} recoveryStatus={recoveryStatus}
        notice={notice} setNotice={setNotice}
        createProjectFromWelcome={projectOps.createProjectFromWelcome} openProject={projectOps.openProject}
        recoverProjectFromWelcome={projectOps.recoverProjectFromWelcome}
      />
    );
  }

  if (projectOps.appStage === "loading") {
    return (
      <div className="loading-stage">
        <div className="loading-card">
          <div className="loading-spinner" />
          <h2>{projectOps.loadingMessage}</h2>
          <p>Easy Edits Pro is preparing your creative session…</p>
        </div>
      </div>
    );
  }

  const timelineWidth = Math.max(1200, timelineEndSeconds * timelineOps.timelineZoom);

  return (
    <div className="app-shell" tabIndex={0} onKeyDown={shortcuts}>
      <AppHeader
        project={project}
        projectState={projectState}
        commands={effectiveCommands}
        busy={busy}
        undoStatus={undoStatus}
        redoStatus={redoStatus}
        selectedClip={timelineOps.selectedClip}
        selectedMedia={selectedMedia}
        snapEnabled={timelineOps.snapEnabled}
        workspace={projectOps.workspace as any}
        davinciWorkspace={projectOps.davinciWorkspace}
        selectDavinciWorkspace={selectDavinciWorkspace}
        invokeMenuCommand={dispatchCommand}
        ensureDiscardIsOkay={projectOps.ensureDiscardIsOkay}
        setNewProjectName={projectOps.setNewProjectName}
        setNewProjectOpen={projectOps.setNewProjectOpen}
        openProject={projectOps.openProject}
        saveProject={projectOps.saveProject}
        importMedia={mediaOps.importMedia}
        setNotice={setNotice}
        collabUser={projectOps.collabUser}
        setCollabModalOpen={projectOps.setCollabModalOpen}
        activeJobs={jobsHook.activeJobs}
        jobs={jobsHook.jobs}
        cancelJob={jobsHook.cancelJob}
        cancelAllJobs={jobsHook.cancelAllJobs}
        clearJobHistory={jobsHook.clearJobHistory}
        setBatchQueueDrawerOpen={mediaOps.setBatchQueueDrawerOpen}
        setAiChatDrawerOpen={mediaOps.setAiChatDrawerOpen}
        theme={theme}
        toggleTheme={toggleTheme}
        fullscreen={projectOps.fullscreen}
        toggleFullscreen={projectOps.toggleFullscreen}
      />

      <ErrorBoundary fallbackTitle="Workspace Rendering Issue" onReset={() => projectOps.setDavinciWorkspace("edit")}>
        <WorkspaceRouter
          davinciWorkspace={projectOps.davinciWorkspace}
          project={project} media={media} getMediaUrl={getMediaUrl}
          selectedTimelineClip={selectedTimelineClip}
          selectedMedia={selectedMedia}
          missingMediaIds={missingMediaIds} busy={busy}
          timelineWidth={timelineWidth} timelineEndSeconds={timelineEndSeconds}
          selectDavinciWorkspace={selectDavinciWorkspace} playback={playback}
          mediaOps={mediaOps} timelineOps={timelineOps}
          inspectorOps={inspectorOps} projectOps={projectOps}
          invokeMenuCommand={dispatchCommand}
          perform={perform} undoStatus={undoStatus} redoStatus={redoStatus}
          programPreviewError={programPreviewError} audioClockMaster={audioClockMaster}
          audioOps={audioOps}
        />
      </ErrorBoundary>

      <StatusBar
        busy={busy} notice={notice} connection={connection}
        toolLabel={activeTool?.label}
        editDetail={
          timelineOps.trimSession
            ? formatTrimStatus(timelineOps.trimSession)
            : undefined
        }
        onFetchGpuTelemetry={projectOps.handleFetchGpuTelemetry}
        onToggleGpuTelemetry={() => projectOps.setGpuTelemetryOpen((v) => !v)}
      />

      <PreferencesDialog
        open={preferencesOpen}
        onClose={() => setPreferencesOpen(false)}
        preferences={preferences}
        onChange={(patch) =>
          setPreferences((current) => ({ ...current, ...patch }))
        }
        onReset={() => {
          setPreferences(DEFAULT_PREFERENCES);
          timelineOps.setSnapEnabled(DEFAULT_PREFERENCES.snap);
          timelineOps.setTimelineTrackHeight(DEFAULT_PREFERENCES.trackHeight);
        }}
        snapEnabled={timelineOps.snapEnabled}
        onSnapChange={timelineOps.setSnapEnabled}
        trackHeight={timelineOps.timelineTrackHeight}
        onTrackHeightChange={timelineOps.setTimelineTrackHeight}
      />

      <LayoutPresetsDialog
        open={presetsOpen}
        onClose={() => setPresetsOpen(false)}
        activeWorkspace={projectOps.workspace}
        customPresets={customPresets}
        onApply={applyWorkspacePreset}
        onSaveCurrent={saveCurrentPreset}
        onRename={renameCustomPreset}
        onDelete={deleteCustomPreset}
        onResetActive={() => {
          window.dispatchEvent(new Event(PANEL_RESET_EVENT));
          setNotice(`Reset the ${projectOps.workspace} panel layout`);
          setPresetsOpen(false);
        }}
      />

      <ShortcutsDialog
        open={shortcutsOpen}
        onClose={() => setShortcutsOpen(false)}
        commands={effectiveCommands}
        tools={timelineTools.map((tool) => ({
          id: tool.id,
          label: tool.label,
          shortcut: tool.shortcut,
        }))}
        overrides={shortcutOverrides}
        onChange={(id, shortcut) =>
          setShortcutOverrides((current) => ({ ...current, [id]: shortcut }))
        }
        onImport={(next) => setShortcutOverrides(next)}
        onResetAll={() => setShortcutOverrides({})}
      />

      <EditorModals
        projectOps={projectOps} mediaOps={mediaOps}
        timelineOps={timelineOps} project={project}
        media={media} busy={busy}
        refreshProject={refreshProject} setNotice={setNotice}
      />
    </div>
  );
}

export function App() {
  return (
    <ProjectProvider>
      <EditorApp />
    </ProjectProvider>
  );
}

export default App;
