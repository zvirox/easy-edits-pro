import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProjectProvider, runCommand } from "./app/ProjectProvider";
import { useProject } from "./app/useProject";
import { usePlayback } from "./features/preview";
import { useEncoderCapabilities, useJobs } from "./app/useJobs";
import { useRecovery, ProjectWelcomeScreen } from "./features/welcome";
import { useMediaServer } from "./features/media-pool";
import { useShortcuts, type DavinciWorkspace } from "./app/useShortcuts";
import { useMediaOperations } from "./app/useMediaOperations";
import { useTimelineOperations } from "./app/useTimelineOperations";
import { pickFirstTimelineClip, setPrimaryClip } from "./app/selectionStore";
import { useInspectorOperations } from "./app/useInspectorOperations";
import { useProjectOperations } from "./app/useProjectOperations";
import { useAudioWorkspace } from "./app/useAudioWorkspace";
import { useMarkers } from "./app/useMarkers";
import { useCaptions } from "./app/useCaptions";
import { useGraphics } from "./app/useGraphics";
import { useNodeGraph } from "./app/useNodeGraph";
import { useShortcutsSync } from "./app/useShortcutsSync";
import { applyViewCommand } from "./app/viewCommands";

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
import { parseTimecode, resolvePlayheadCommand } from "./app/playheadNavigation";
import { inspectSelectedEdit } from "./app/editInspect";
import {
  EDIT_CLIPBOARD_COMMANDS,
  useEditClipboard,
} from "./app/useEditClipboard";
import type { CommandSpec, SelectedClip } from "./types";
import type { ProjectDocument } from "./app/projectTypes";

function EditorApp() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const preferencesHydrated = useRef(false);
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
    if (!preferencesHydrated.current) return;
    void runCommand("prefs.set", { values: preferences }).catch(() => {});
  }, [preferences]);

  // Settings live in the engine's settings.json; localStorage stays as the
  // instant cache so the first paint still has the user's theme and glass mode.
  useEffect(() => {
    let active = true;
    void runCommand<{ prefs: Record<string, unknown> }>("prefs.get")
      .then(({ prefs }) => {
        if (!active) return;
        const allowed = new Set(Object.keys(DEFAULT_PREFERENCES));
        const stored = Object.fromEntries(
          Object.entries(prefs ?? {}).filter(([key]) => allowed.has(key)),
        ) as Partial<Preferences>;
        if (Object.keys(stored).length) {
          setPreferences((current) => ({ ...current, ...stored }));
        }
      })
      .catch(() => {})
      .finally(() => {
        preferencesHydrated.current = true;
      });
    return () => {
      active = false;
    };
  }, []);

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
  // G2 purged the floating-panel engine; wipe the legacy persisted MDI
  // layouts (localStorage mirror of the removed save/load commands).
  useEffect(() => {
    const legacy: string[] = [];
    for (let i = 0; i < localStorage.length; i += 1) {
      const key = localStorage.key(i);
      if (key && key.startsWith("easy-edits-pro.panel-layout.")) legacy.push(key);
    }
    legacy.forEach((key) => localStorage.removeItem(key));
  }, []);
  const { shortcutOverrides, setShortcutOverrides } = useShortcutsSync();

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
  const encoderInfo = useEncoderCapabilities();
  const { mediaServerPort, mediaServerToken, mediaServerReady, getMediaUrl } = useMediaServer();
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

  const menuContext = useRef<{
    selectedClip: SelectedClip | null;
    selectedClips: SelectedClip[];
    playhead: number;
    project: ProjectDocument["project"] | null;
  }>({ selectedClip: null, selectedClips: [], playhead: 0, project: null });

  const projectOps = useProjectOperations({
    projectState, setProjectState, refreshProject,
    perform, runCommand, setNotice, setBusy, busy,
    setPlayhead: playback.setPlayhead,
    setSelectedClip: () => {}, setSelectedClips: () => {},
    setSelectedTransition: () => {}, setSelectedMediaId: () => {},
  getMenuContext: () => ({
    selectedClip: menuContext.current.selectedClip,
    selectedClips: menuContext.current.selectedClips,
    playhead: menuContext.current.playhead,
    project: menuContext.current.project,
  }),
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
    seekProgramTo: playback.seekProgramTo,
    handleShuttleKey: playback.handleShuttleKey,
    stopPlayback: playback.stopPlayback,
    timelineEndSeconds,
    perform, runCommand, setNotice, busy, refreshProject,
    setCompoundClipModalOpen: projectOps.setCompoundClipModalOpen,
    setCompoundClipNameDraft: projectOps.setCompoundClipNameDraft,
    keyframeProperty: "opacity", keyframeValue: "1",
    initialSnapEnabled: preferences.snap,
    initialTrackHeight: preferences.trackHeight,
  });

  const markerOps = useMarkers({
    project,
    playhead: playback.playhead,
    runCommand,
    refreshProject,
    setNotice,
    seek: playback.seekProgramTo,
  });

  const captionsOps = useCaptions({
    project,
    playhead: playback.playhead,
    runCommand,
    refreshProject,
    setNotice,
    seek: playback.seekProgramTo,
  });

  const graphicsOps = useGraphics({
    project,
    playhead: playback.playhead,
    runCommand,
    refreshProject,
    setNotice,
  });

  const nodeGraphOps = useNodeGraph({
    project,
    runCommand,
    refreshProject,
    setNotice,
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
      selectedClips: timelineOps.selectedClips,
      playhead: playback.playhead,
      project,
    };
  }, [timelineOps.selectedClip, timelineOps.selectedClips, playback.playhead, project]);

  // F1: auto-select the first V1 clip once per sequence load so the
  // Inspector, Effects, Color, and Curves docks come up live without a
  // manual click. Per-sequence guard avoids re-selecting after a delete.
  const autoSelectedSequenceRef = useRef<number | null>(null);
  useEffect(() => {
    const sequenceId = project?.sequence.id;
    if (sequenceId == null) return;
    if (autoSelectedSequenceRef.current === sequenceId) return;
    autoSelectedSequenceRef.current = sequenceId;
    if (timelineOps.selectedClip) return;
    const first = pickFirstTimelineClip(project ?? null);
    if (first) setPrimaryClip(first);
  }, [project, timelineOps.selectedClip]);

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

  const runEditClipboard = useEditClipboard({
    project,
    media,
    playhead: playback.playhead,
    selectedClip: timelineOps.selectedClip,
    selectedClips: timelineOps.selectedClips,
    setSelectedClip: timelineOps.setSelectedClip,
    setSelectedClips: timelineOps.setSelectedClips,
    runCommand,
    refreshProject,
    setNotice,
  });

  /**
   * Single entry point for menus, the command palette, keyboard shortcuts, and
   * workspace toolbars. UI-only commands are handled here; everything else goes
   * through the engine-aware `invokeMenuCommand`.
   */
  const dispatchCommand = useCallback(
    async (id: string) => {
      if (id === "marker.panel" || id === "caption.panel" || id === "graphic.panel") {
        // The docked-adjacent panels live in the Edit workspace; switching
        // there and announcing the panel keeps the gesture one step.
        selectDavinciWorkspace("edit");
        if (id === "marker.panel") markerOps.openPanel();
        else if (id === "caption.panel") captionsOps.openPanel();
        else graphicsOps.openPanel();
        return;
      }
      if (id.startsWith("caption.") && (await captionsOps.handleMenuCommand(id))) {
        return;
      }
      if (id.startsWith("graphic.") && (await graphicsOps.handleMenuCommand(id))) {
        return;
      }
      if (id.startsWith("marker.") && (await markerOps.handleMenuCommand(id))) {
        return;
      }
      if (id.startsWith("nodegraph.") && (await nodeGraphOps.handleMenuCommand(id))) {
        return;
      }
      if ((EDIT_CLIPBOARD_COMMANDS as string[]).includes(id)) {
        await runEditClipboard(id as (typeof EDIT_CLIPBOARD_COMMANDS)[number]);
        return;
      }
      if (id === "playhead.go_to_timecode") {
        const value = window.prompt("Go to timecode (HH:MM:SS:FF)", "00:00:00:00");
        if (value === null) return;
        const target = parseTimecode(value, project?.settings.frame_rate);
        if (target === null) {
          setNotice("Enter a timecode such as 00:01:23:12");
          return;
        }
        playback.seekProgramTo(target);
        return;
      }
      if (id.startsWith("playhead.")) {
        const target = resolvePlayheadCommand(id, {
          project,
          selected: timelineOps.selectedClip,
          playhead: playback.playhead,
        });
        if (target !== null) playback.seekProgramTo(target);
        return;
      }
      if (id === "trim.enter") {
        timelineOps.enterTrimMode();
        return;
      }
      if (id === "trim.commit") {
        await timelineOps.commitTrimMode();
        return;
      }
      if (id === "trim.exit" || id === "trim.cancel") {
        timelineOps.cancelTrimMode();
        return;
      }
      if (id === "timeline.inspect" || id === "trim.inspect") {
        const selected = timelineOps.selectedClip;
        if (!selected) {
          setNotice("Select a clip to inspect its edit points");
          return;
        }
        try {
          setNotice(await inspectSelectedEdit(runCommand, id, selected));
        } catch (error) {
          setNotice(String(error));
        }
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
        case "project.update_media":
          await mediaOps.updateMediaMetadata();
          return;
        case "edit.preferences":
          setPreferencesOpen(true);
          return;
        case "help.shortcuts":
          setShortcutsOpen(true);
          return;
      }
      if (
        applyViewCommand(id, {
          preferences,
          setPreferences,
          setPreferencesOpen,
          setShortcutsOpen,
          setNotice,
          setSnapEnabled: timelineOps.setSnapEnabled,
          zoomTimeline: timelineOps.zoomTimeline,
          setTimelineZoom: timelineOps.setTimelineZoom,
        })
      ) {
        return;
      }
      await projectOps.invokeMenuCommand(id);
    },
    [selectDavinciWorkspace, projectOps, mediaOps, timelineOps, markerOps, project, playback, runEditClipboard]
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
    switchMulticamAngle: (angle: number) => void projectOps.switchMulticamAngle(angle),
    handleShuttleKey: playback.handleShuttleKey,
    stepProgramFrame: playback.stepProgramFrame,
    trimActive: timelineOps.trimMode !== null,
    handleTrimShuttle: timelineOps.handleTrimShuttle,
    commitTrim: timelineOps.commitTrimMode,
    cancelTrim: timelineOps.cancelTrimMode,
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

  // F2: the media server port/token must resolve before the editor mounts,
  // otherwise recovered projects build stream URLs with port 0 and every
  // preview silently falls back to an unstreamable source. The init is a
  // local socket probe - this gate settles in milliseconds.
  if (!mediaServerReady) {
    return (
      <div className="loading-stage">
        <div className="loading-card">
          <div className="loading-spinner" />
          <h2>Preparing media engine…</h2>
          <p>Easy Edits Pro is starting the local media server.</p>
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

      <div className="app-workspace-region">
        <ErrorBoundary fallbackTitle="Workspace Rendering Issue" onReset={() => projectOps.setDavinciWorkspace("edit")}>
        <WorkspaceRouter
          commands={commands}
          encoderInfo={encoderInfo}
          davinciWorkspace={projectOps.davinciWorkspace}
          project={project} media={media} getMediaUrl={getMediaUrl}
          selectedTimelineClip={selectedTimelineClip}
          selectedMedia={selectedMedia}
          missingMediaIds={missingMediaIds} busy={busy}
          mediaServerPort={mediaServerPort}
          relinkMedia={(mediaId) => mediaOps.relinkMedia(mediaId)}
          timelineWidth={timelineWidth} timelineEndSeconds={timelineEndSeconds}
          selectDavinciWorkspace={selectDavinciWorkspace} playback={playback}
          mediaOps={mediaOps} timelineOps={timelineOps}
          inspectorOps={inspectorOps} projectOps={projectOps}
          invokeMenuCommand={dispatchCommand}
          perform={perform} undoStatus={undoStatus} redoStatus={redoStatus}
          programPreviewError={programPreviewError} audioClockMaster={audioClockMaster}
          audioOps={audioOps}
          markerOps={markerOps}
          captionsOps={captionsOps}
          nodeGraphOps={nodeGraphOps}
          graphicsOps={graphicsOps}
        />
        </ErrorBoundary>

        {/* EditorModals hosts the AI Assistant floating panel; it lives inside
            the workspace region so the panel can never cover app chrome. */}
        <EditorModals
          projectOps={projectOps} mediaOps={mediaOps}
          timelineOps={timelineOps} project={project}
          media={media} busy={busy}
          refreshProject={refreshProject} setNotice={setNotice}
        />
      </div>

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
          void runCommand("prefs.reset").catch(() => {});
        }}
        snapEnabled={timelineOps.snapEnabled}
        onSnapChange={timelineOps.setSnapEnabled}
        trackHeight={timelineOps.timelineTrackHeight}
        onTrackHeightChange={timelineOps.setTimelineTrackHeight}
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
