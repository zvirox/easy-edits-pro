import { useCallback, useEffect, useState } from "react";
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

function EditorApp() {
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    try {
      const saved = localStorage.getItem("eep-theme");
      if (saved === "light" || saved === "dark") return saved;
    } catch {}
    return "dark";
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.documentElement.classList.toggle("light", theme === "light");
    try {
      localStorage.setItem("eep-theme", theme);
    } catch {}
  }, [theme]);

  const toggleTheme = useCallback(() => setTheme((t) => (t === "dark" ? "light" : "dark")), []);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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

  const projectOps = useProjectOperations({
    projectState, setProjectState, refreshProject,
    perform, runCommand, setNotice, setBusy, busy,
    setPlayhead: playback.setPlayhead,
    setSelectedClip: () => {}, setSelectedClips: () => {},
    setSelectedTransition: () => {}, setSelectedMediaId: () => {},
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
    perform, setNotice, busy, refreshProject,
    setCompoundClipModalOpen: projectOps.setCompoundClipModalOpen,
    setCompoundClipNameDraft: projectOps.setCompoundClipNameDraft,
    keyframeProperty: "opacity", keyframeValue: "1",
  });

  const audioOps = useAudioWorkspace(project, perform, playback.programPlaying);

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

  const shortcuts = useShortcuts({
    commands: [],
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
    invokeMenuCommand: projectOps.invokeMenuCommand,
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

  const timelineWidth = Math.max(1200, timelineEndSeconds * timelineOps.timelineZoom);

  return (
    <div className="app-shell" tabIndex={0} onKeyDown={shortcuts}>
      <AppHeader
        project={project}
        projectState={projectState}
        commands={[]}
        busy={busy}
        undoStatus={undoStatus}
        redoStatus={redoStatus}
        selectedClip={timelineOps.selectedClip}
        selectedMedia={selectedMedia}
        snapEnabled={timelineOps.snapEnabled}
        workspace={projectOps.workspace as any}
        davinciWorkspace={projectOps.davinciWorkspace}
        selectDavinciWorkspace={selectDavinciWorkspace}
        invokeMenuCommand={projectOps.invokeMenuCommand}
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
          perform={perform} undoStatus={undoStatus} redoStatus={redoStatus}
          programPreviewError={programPreviewError} audioClockMaster={audioClockMaster}
          audioOps={audioOps}
        />
      </ErrorBoundary>

      <StatusBar
        busy={busy} notice={notice} connection={connection}
        onFetchGpuTelemetry={projectOps.handleFetchGpuTelemetry}
        onToggleGpuTelemetry={() => projectOps.setGpuTelemetryOpen((v) => !v)}
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
