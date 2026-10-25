import {
  type Dispatch,
  type SetStateAction,
  useCallback,
  useState,
} from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import type {
  DavinciWorkspace,
  GpuMetrics,
  ProjectDocument,
  ProjectStatus,
  SelectedClip,
  Transition,
} from "./projectTypes";
import { TICKS_PER_SECOND } from "./projectTypes";import { buildBatchCommandParams } from "./menuCommandParams";
import { useSpeechOperations } from "./useSpeechOperations";

const recentProjectsKey = "easy-edits-pro.recent-projects.v1";
const projectFilters = [{ name: "Easy Edits Pro project", extensions: ["saturn"] }];

export type RecentProject = { name: string; path: string };
export type AppStage = "welcome" | "loading" | "editor";

export const resolutionPresets = [
  { value: "1920x1080", label: "Full HD · 1920 × 1080", width: 1920, height: 1080 },
  { value: "3840x2160", label: "Ultra HD · 3840 × 2160", width: 3840, height: 2160 },
  { value: "2560x1440", label: "QHD · 2560 × 1440", width: 2560, height: 1440 },
  { value: "1280x720", label: "HD · 1280 × 720", width: 1280, height: 720 },
  { value: "1080x1920", label: "Vertical · 1080 × 1920", width: 1080, height: 1920 },
  { value: "1080x1080", label: "Square · 1080 × 1080 (1:1)", width: 1080, height: 1080 },
  { value: "custom", label: "Custom resolution", width: 1920, height: 1080 },
];

export const frameRateOptions = [
  { value: "24000/1001", label: "23.976 fps", numerator: 24_000, denominator: 1_001 },
  { value: "24/1", label: "24 fps", numerator: 24, denominator: 1 },
  { value: "25/1", label: "25 fps", numerator: 25, denominator: 1 },
  { value: "30000/1001", label: "29.97 fps", numerator: 30_000, denominator: 1_001 },
  { value: "30/1", label: "30 fps", numerator: 30, denominator: 1 },
  { value: "50/1", label: "50 fps", numerator: 50, denominator: 1 },
  { value: "60000/1001", label: "59.94 fps", numerator: 60_000, denominator: 1_001 },
  { value: "60/1", label: "60 fps", numerator: 60, denominator: 1 },
];

export interface UseProjectOperationsOptions {
  projectState: ProjectStatus | null;
  setProjectState: (status: ProjectStatus) => void;
  refreshProject: () => Promise<ProjectStatus>;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  setNotice: (msg: string) => void;
  setBusy: (busy: boolean) => void;
  busy: boolean;
  setPlayhead: (p: number) => void;
  setSelectedClip: Dispatch<SetStateAction<SelectedClip | null>>;
  setSelectedClips: Dispatch<SetStateAction<SelectedClip[]>>;
  setSelectedTransition: Dispatch<SetStateAction<{ track_id: number; transition: Transition } | null>>;
  setSelectedMediaId: Dispatch<SetStateAction<number | null>>;
  /** Live timeline context, read at invocation time to fill command parameters. */
  getMenuContext: () => {
    selectedClip: SelectedClip | null;
    selectedClips: SelectedClip[];
    playhead: number;
    project: ProjectDocument["project"] | null;
  };
}

export function useProjectOperations({
  projectState,
  setProjectState: _setProjectState,
  refreshProject,
  perform,
  runCommand,
  setNotice,
  setBusy,
  busy: _busy,
  setPlayhead,
  setSelectedClip,
  setSelectedClips,
  setSelectedTransition,
  setSelectedMediaId,
  getMenuContext,
}: UseProjectOperationsOptions) {
  const project = projectState?.document.project ?? null;
  const speechOps = useSpeechOperations({ project, refreshProject, setNotice, setBusy });

  const [appStage, setAppStage] = useState<AppStage>("welcome");
  const [loadingMessage, setLoadingMessage] = useState("Preparing your editing workspace");
  const [welcomeMode, setWelcomeMode] = useState<"home" | "new-project">("home");

  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(() => {
    try {
      const recent = JSON.parse(localStorage.getItem(recentProjectsKey) ?? "[]") as RecentProject[];
      return Array.isArray(recent)
        ? recent.filter((item) => typeof item?.name === "string" && typeof item?.path === "string").slice(0, 6)
        : [];
    } catch {
      return [];
    }
  });

  const [setupProjectName, setSetupProjectName] = useState("Untitled project");
  const [setupSequenceName, setSetupSequenceName] = useState("Sequence 1");
  const [setupResolution, setSetupResolution] = useState("1920x1080");
  const [setupWidth, setSetupWidth] = useState("1920");
  const [setupHeight, setSetupHeight] = useState("1080");
  const [setupFrameRate, setSetupFrameRate] = useState("30/1");

  const [workspace, setWorkspace] = useState<string>("edit");
  const [davinciWorkspace, setDavinciWorkspace] = useState<DavinciWorkspace>("edit");

  const [newProjectName, setNewProjectName] = useState("Untitled project");
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [projectJsonOpen, setProjectJsonOpen] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);

  // Export settings
  const [exportFormat, setExportFormat] = useState<"mp4" | "webm">("mp4");
  const [exportResolution, setExportResolution] = useState("source");
  const [exportFrameRate, setExportFrameRate] = useState("source");
  const [exportMarkedRange, setExportMarkedRange] = useState(false);
  const [exportFileName, setExportFileName] = useState("");
  const [exportOutputPath, setExportOutputPath] = useState("");
  const [useHardwareEncoding, setUseHardwareEncoding] = useState(true);
  const [rendering, setRendering] = useState(false);
  const [renderProgress, setRenderProgress] = useState<{ fractionPercent: number; phase: string } | null>(null);

  // Interop modal
  const [interopModalOpen, setInteropModalOpen] = useState(false);
  const [interopMode, setInteropMode] = useState<"export" | "import">("export");
  const [interopFormat, setInteropFormat] = useState<"fcpxml" | "premiere_xml" | "edl" | "otio">("fcpxml");
  const [interopContent, setInteropContent] = useState("");
  const [interopProjectName, setInteropProjectName] = useState("Interop Sequence");
  const [isInteropBusy, setIsInteropBusy] = useState(false);

  // Compound clip modal
  const [compoundClipModalOpen, setCompoundClipModalOpen] = useState(false);
  const [compoundClipNameDraft, setCompoundClipNameDraft] = useState("");

  // Clip speed and duration
  const [speedDurationOpen, setSpeedDurationOpen] = useState(false);

  // GPU Telemetry
  const [gpuTelemetryOpen, setGpuTelemetryOpen] = useState(false);
  const [gpuMetrics, setGpuMetrics] = useState<GpuMetrics | null>(null);

  const rememberProject = useCallback((name: string, path: string) => {
    setRecentProjects((current) => {
      const next = [{ name, path }, ...current.filter((item) => item.path !== path)].slice(0, 6);
      try {
        localStorage.setItem(recentProjectsKey, JSON.stringify(next));
      } catch {}
      return next;
    });
  }, []);

  const ensureDiscardIsOkay = useCallback(async () => {
    if (!projectState?.is_dirty) return true;
    return confirm("Discard unsaved changes?", {
      title: "Easy Edits Pro",
      kind: "warning",
    });
  }, [projectState?.is_dirty]);

  const loadProjectFromPath = useCallback(
    async (path: string) => {
      const previousStage = appStage;
      setLoadingMessage("Opening project and preparing the timeline");
      setAppStage("loading");
      setNotice("");
      try {
        const result = await runCommand<{ name: string }>("file.open", { path });
        setSelectedClip(null);
        setSelectedClips([]);
        setSelectedTransition(null);
        setSelectedMediaId(null);
        setPlayhead(0);
        await refreshProject();
        rememberProject(result.name, path);
        setNotice("Project opened");
        setAppStage("editor");
      } catch (error) {
        setAppStage(previousStage === "editor" ? "editor" : "welcome");
        setNotice(String(error));
        throw error;
      }
    },
    [appStage, runCommand, setSelectedClip, setSelectedClips, setSelectedTransition, setSelectedMediaId, setPlayhead, refreshProject, rememberProject, setNotice]
  );

  const openProject = useCallback(
    async (path?: string) => {
      if (appStage === "editor" && !(await ensureDiscardIsOkay())) return;
      const selectedPath =
        path ??
        (await open({
          title: "Open Easy Edits Pro project",
          multiple: false,
          filters: projectFilters,
        }));
      if (typeof selectedPath !== "string") return;
      await loadProjectFromPath(selectedPath);
    },
    [appStage, ensureDiscardIsOkay, loadProjectFromPath]
  );

  const createProjectFromWelcome = useCallback(async () => {
    const name = setupProjectName.trim();
    const sequenceName = setupSequenceName.trim();
    const width = Number(setupWidth);
    const height = Number(setupHeight);
    const frameRate = frameRateOptions.find((option) => option.value === setupFrameRate) ?? frameRateOptions[4];
    if (
      !name ||
      !sequenceName ||
      !Number.isInteger(width) ||
      !Number.isInteger(height) ||
      width <= 0 ||
      height <= 0 ||
      width > 16_384 ||
      height > 16_384
    ) {
      throw new Error("Enter project and timeline names and dimensions between 1 and 16,384 pixels.");
    }
    const selectedPath = await save({
      title: "Create Easy Edits Pro project",
      defaultPath: `${name.replace(/[\\/:*?"<>|]/g, "-")}.saturn`,
      filters: projectFilters,
    });
    if (!selectedPath) return;
    const path = selectedPath.toLowerCase().endsWith(".saturn") ? selectedPath : `${selectedPath}.saturn`;

    setLoadingMessage("Creating your project and setting up the timeline");
    setAppStage("loading");
    setNotice("");
    try {
      await runCommand("project.new", {
        name,
        sequence_name: sequenceName,
        width,
        height,
        frame_rate: { numerator: frameRate.numerator, denominator: frameRate.denominator },
      });
      await runCommand("file.save", { path });
      setSelectedClip(null);
      setSelectedClips([]);
      setSelectedTransition(null);
      setSelectedMediaId(null);
      setPlayhead(0);
      await refreshProject();
      rememberProject(name, path);
      setNotice("Project created");
      setWelcomeMode("home");
      setAppStage("editor");
    } catch (error) {
      setAppStage("welcome");
      setNotice(String(error));
      throw error;
    }
  }, [setupProjectName, setupSequenceName, setupWidth, setupHeight, setupFrameRate, runCommand, setSelectedClip, setSelectedClips, setSelectedTransition, setSelectedMediaId, setPlayhead, refreshProject, rememberProject, setNotice]);

  const recoverProjectFromWelcome = useCallback(async () => {
    setLoadingMessage("Restoring your recovered project");
    setAppStage("loading");
    setNotice("");
    try {
      await invoke("restore_autosave");
      const status = await refreshProject();
      if (status.path) rememberProject(status.document.project.name, status.path);
      setNotice("Autosaved project recovered. Save it to keep your changes.");
      setAppStage("editor");
    } catch (error) {
      setAppStage("welcome");
      setNotice(String(error));
      throw error;
    }
  }, [refreshProject, rememberProject, setNotice]);

  const createProject = useCallback(async () => {
    const name = newProjectName.trim();
    if (!name) return;
    setNewProjectOpen(false);
    setSelectedMediaId(null);
    setSelectedClip(null);
    setSelectedClips([]);
    setSelectedTransition(null);
    setPlayhead(0);
    await perform("project.new", { name }, "New project created");
  }, [newProjectName, setSelectedMediaId, setSelectedClip, setSelectedClips, setSelectedTransition, setPlayhead, perform]);

  const saveProject = useCallback(async () => {
    if (!project) return;
    let path = projectState?.path ?? null;
    if (!path) {
      path = await save({
        title: "Save Easy Edits Pro project",
        defaultPath: `${project.name.replace(/[\\/:*?"<>|]/g, "-")}.saturn`,
        filters: projectFilters,
      });
    }
    if (!path) return;
    await perform("file.save", { path }, "Project saved");
  }, [project, projectState?.path, perform]);

  const chooseExportDestination = useCallback(async () => {
    if (!project || rendering) return null;
    const extension = exportFormat;
    const safeName = (exportFileName.trim() || project.name).replace(/[\\/:*?"<>|]/g, "-");
    const path = await save({
      title: "Export video",
      defaultPath: exportOutputPath || `${safeName}.${extension}`,
      filters: [{ name: extension === "mp4" ? "MP4 video" : "WebM video", extensions: [extension] }],
    });
    if (path) {
      setExportOutputPath(path);
      const filename = path.split(/[\\/]/).at(-1) ?? "";
      setExportFileName(filename.replace(/\.[^.]+$/, ""));
    }
    return path;
  }, [project, rendering, exportFormat, exportFileName, exportOutputPath]);

  const exportProject = useCallback(async () => {
    if (!project || rendering) return;
    const path = exportOutputPath || (await chooseExportDestination());
    if (!path) return;
    setExportOutputPath(path);

    const resolution =
      exportResolution === "source"
        ? { width: project.settings.width, height: project.settings.height }
        : resolutionPresets.find((item) => item.value === exportResolution) ?? {
            width: project.settings.width,
            height: project.settings.height,
          };
    const frameRate =
      exportFrameRate === "source"
        ? project.settings.frame_rate
        : (() => {
            const [numerator, denominator] = exportFrameRate.split("/").map(Number);
            return { numerator, denominator };
          })();

    const channel = new Channel<{ fractionPercent: number; phase: string }>();
    channel.onmessage = (progress) => setRenderProgress(progress);
    setRendering(true);
    setRenderProgress({ fractionPercent: 0, phase: "Preparing timeline" });
    setNotice("");
    try {
      await invoke("start_export", {
        outputPath: path,
        useMarkRange: exportMarkedRange,
        settings: {
          width: resolution.width,
          height: resolution.height,
          frameRateNumerator: frameRate.numerator,
          frameRateDenominator: frameRate.denominator,
          useHardwareEncoding,
        },
        onProgress: channel,
      });
      setRenderProgress({ fractionPercent: 100, phase: "Complete" });
      setNotice(`Export complete: ${path}`);
    } catch (error) {
      setNotice(`Export failed: ${String(error)}`);
    } finally {
      setRendering(false);
    }
  }, [
    project,
    rendering,
    exportOutputPath,
    chooseExportDestination,
    exportResolution,
    exportFrameRate,
    exportMarkedRange,
    useHardwareEncoding,
    setNotice,
  ]);

  const handleExportTimelineInterop = useCallback(
    async (fmt?: string) => {
      const targetFormat = fmt || interopFormat;
      setIsInteropBusy(true);
      try {
        const content = await invoke<string>("export_timeline_interop", { format: targetFormat, path: null });
        setInteropContent(content);
        setNotice(`Generated ${targetFormat.toUpperCase()} interchange data`);
      } catch (e) {
        setNotice(`Interchange export error: ${String(e)}`);
      } finally {
        setIsInteropBusy(false);
      }
    },
    [interopFormat, setNotice]
  );

  const handleImportTimelineInterop = useCallback(async () => {
    const selected = await open({
      title: "Import Timeline File",
      multiple: false,
      filters: [
        { name: "Timeline Formats", extensions: ["xml", "fcpxml", "edl", "otio"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (!selected || typeof selected !== "string") return;
    setIsInteropBusy(true);
    try {
      await invoke("import_timeline_interop", {
        format: interopFormat,
        content: null,
        path: selected,
        projectName: interopProjectName,
      });
      await refreshProject();
      setInteropModalOpen(false);
      setNotice(`Imported timeline file: ${selected}`);
    } catch (e) {
      setNotice(`Import error: ${String(e)}`);
    } finally {
      setIsInteropBusy(false);
    }
  }, [interopFormat, interopProjectName, refreshProject, setNotice]);

  const handleFetchGpuTelemetry = useCallback(async () => {
    try {
      const metrics = await invoke<GpuMetrics>("get_presentation_metrics");
      setGpuMetrics(metrics);
    } catch {
      setGpuMetrics({ fpsMeasured: 60.0, surfaceWidth: 1920, surfaceHeight: 1080 });
    }
  }, []);

  const toggleFullscreen = useCallback(async () => {
    try {
      const win = getCurrentWindow();
      const next = !fullscreen;
      await win.setFullscreen(next);
      setFullscreen(next);
    } catch {}
  }, [fullscreen]);

  /** Switches the selected timeline clip's multicam angle (monitor cuts, 1-9 keys). */
  const switchMulticamAngle = useCallback(
    async (angleId: number) => {
      const { selectedClip } = getMenuContext();
      if (!selectedClip) {
        setNotice("Select a timeline clip to switch its multicam angle");
        return;
      }
      await perform(
        "multicam.switch_angle",
        {
          track_id: selectedClip.track_id,
          clip_index: selectedClip.clip_index,
          angle_id: angleId,
        },
        "Multicam angle switched",
      );
    },
    [getMenuContext, perform, setNotice],
  );

  /**
   * Cross-correlates the waveforms of the first multicam group's angles and
   * applies the resulting offsets through `multicam.sync`.
   */
  const syncMulticamWaveforms = useCallback(async () => {
    const group = project?.sequence.multicam_clips?.[0];
    if (!group) {
      setNotice("No multicam clip — create one from Clip > Multicam");
      return;
    }
    const [reference, ...rest] = group.angles;
    if (!reference || rest.length === 0) {
      setNotice("A multicam clip needs at least two angles to sync");
      return;
    }
    const offsets: Record<string, number> = { [String(reference.id)]: 0 };
    let syncedCount = 0;
    for (const angle of rest) {
      try {
        const res = await invoke<{ offsetTicks: number; synced: boolean }>(
          "sync_multicam_by_waveform",
          { referenceMediaId: reference.media_id, targetMediaId: angle.media_id },
        );
        offsets[String(angle.id)] = res.synced ? res.offsetTicks : 0;
        if (res.synced) syncedCount += 1;
      } catch {
        offsets[String(angle.id)] = 0;
      }
    }
    await perform(
      "multicam.sync",
      { multicam_id: group.id, sync_mode: "audio_waveform", offsets },
      syncedCount > 0
        ? `Synced ${syncedCount} angle${syncedCount === 1 ? "" : "s"} by waveform`
        : "Waveforms too dissimilar — offsets left at zero",
    );
  }, [project, perform, setNotice]);

  /**
   * Menu, palette, and toolbar entry point.
   *
   * Commands that need timeline context get their parameters from the current
   * selection and playhead, so a menu item never fires with an empty payload.
   */
  const invokeMenuCommand = useCallback(
    async (id: string, explicitParams?: unknown) => {
      switch (id) {
        case "project.new":
          setNewProjectOpen(true);
          return;
        case "file.open":
          await openProject();
          return;
        case "file.save":
          await saveProject();
          return;
        case "project.inspect":
          setProjectJsonOpen(true);
          return;
        case "speech.transcribe_to_subtitles":
          speechOps.setTranscribeModalOpen(true);
          return;
        case "speech.ripple_delete_silence":
        case "speech.ripple_delete_fillers":
          speechOps.setSilenceModalOpen(true);
          return;
        case "view.fullscreen":
          await toggleFullscreen();
          return;
        case "multicam.create": {
          const assets = (project?.media ?? []).filter(
            (asset) => asset.kind === "video" || asset.kind === "audio",
          );
          if (assets.length < 2) {
            setNotice("Import at least two media assets to build a multicam clip");
            return;
          }
          const requested = window.prompt("Multicam clip name", "Multicam Sequence");
          if (requested === null) return;
          const name = requested.trim() || "Multicam Sequence";
          const angles = assets.slice(0, 9).map((asset, index) => ({
            id: index + 1,
            name: asset.name,
            media_id: asset.id,
            sync_offset_ticks: 0,
          }));
          await perform(
            "multicam.create",
            { name, angles, sync_mode: "audio_waveform" },
            "Multicam clip created",
          );
          return;
        }
        case "timeline.rate_stretch": {
          if (!getMenuContext().selectedClip) {
            setNotice("Select a timeline clip first");
            return;
          }
          setSpeedDurationOpen(true);
          return;
        }
      }

      let params = explicitParams;
      if (params === undefined) {
        const { selectedClip, playhead } = getMenuContext();
        if (id === "sequence.create") {
          const fallback = `Sequence ${(project?.sequences?.length ?? 0) + 2}`;
          const requested = window.prompt("New sequence name", fallback);
          if (requested === null) return;
          params = requested.trim() ? { name: requested.trim() } : {};
        } else if (id === "sequence.rename") {
          const requested = window.prompt(
            "Rename sequence",
            project?.sequence.name ?? "Sequence",
          );
          if (requested === null || !requested.trim()) return;
          params = {
            sequence_id: project?.sequence.id,
            name: requested.trim(),
          };
        } else if (id === "sequence.delete") {
          if (
            !window.confirm(
              `Delete “${project?.sequence.name ?? "this sequence"}”? This can be undone.`,
            )
          ) {
            return;
          }
          params = { sequence_id: project?.sequence.id };
        } else if (id === "timeline.set_mark_in" || id === "timeline.set_mark_out") {
          params = { at: Math.round(playhead * TICKS_PER_SECOND) };
        } else if (selectedClip && id === "timeline.split_clip") {
          params = { ...selectedClip, at: Math.round(playhead * TICKS_PER_SECOND) };
        } else if (
          selectedClip &&
          (id === "timeline.remove_clip" || id === "timeline.ripple_delete")
        ) {
          params = { ...selectedClip };
        } else if (id.startsWith("interop.import_")) {
          const filters: { name: string; extensions: string[] }[] =
            id === "interop.import_edl"
              ? [{ name: "CMX 3600 EDL", extensions: ["edl"] }]
              : id === "interop.import_fcpxml"
                ? [{ name: "Final Cut Pro XML", extensions: ["fcpxml", "xml"] }]
                : id === "interop.import_premiere_xml"
                  ? [{ name: "Premiere Pro XML", extensions: ["xml"] }]
                  : [{ name: "OpenTimelineIO", extensions: ["otio", "json"] }];
          const selectedPath = await open({
            title: "Import timeline file",
            multiple: false,
            filters,
          });
          if (typeof selectedPath !== "string") return;
          const content = await invoke<string>("read_text_file", { path: selectedPath });
          const stem =
            selectedPath.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") || "Imported";
          params =
            id === "interop.import_edl"
              ? { edl: content, project_name: stem }
              : id === "interop.import_otio"
                ? { content, project_name: stem }
                : { xml: content };
        } else {
          const built = buildBatchCommandParams(id, getMenuContext());
          if (built) {
            if (built.notice) setNotice(built.notice);
            if (built.cancel) return;
            params = built.params ?? null;
          }
        }
      }
      const ok = await perform(id, params ?? null);
      if (ok && id.startsWith("interop.import_")) {
        // The import replaced the whole document — reset the editing surface
        // the same way opening a project does.
        setSelectedClip(null);
        setSelectedClips([]);
        setSelectedTransition(null);
        setSelectedMediaId(null);
        setPlayhead(0);
        setAppStage("editor");
        setNotice("Timeline imported");
      }
    },
    [openProject, saveProject, toggleFullscreen, perform, getMenuContext, project]
  );

  return {
    appStage,
    setAppStage,
    loadingMessage,
    welcomeMode,
    setWelcomeMode,
    recentProjects,
    setupProjectName,
    setSetupProjectName,
    setupSequenceName,
    setSetupSequenceName,
    setupResolution,
    setSetupResolution,
    setupWidth,
    setSetupWidth,
    setupHeight,
    setSetupHeight,
    setupFrameRate,
    setSetupFrameRate,
    workspace,
    setWorkspace,
    davinciWorkspace,
    setDavinciWorkspace,
    newProjectName,
    setNewProjectName,
    newProjectOpen,
    setNewProjectOpen,
    projectJsonOpen,
    setProjectJsonOpen,
    runCommand,
    fullscreen,
    exportFormat,
    setExportFormat,
    exportResolution,
    setExportResolution,
    exportFrameRate,
    setExportFrameRate,
    exportMarkedRange,
    setExportMarkedRange,
    exportFileName,
    setExportFileName,
    exportOutputPath,
    setExportOutputPath,
    useHardwareEncoding,
    setUseHardwareEncoding,
    rendering,
    renderProgress,
    ...speechOps,
    interopModalOpen,
    setInteropModalOpen,
    interopMode,
    setInteropMode,
    interopFormat,
    setInteropFormat,
    interopContent,
    setInteropContent,
    interopProjectName,
    setInteropProjectName,
    isInteropBusy,
    setIsInteropBusy,
    compoundClipModalOpen,
    setCompoundClipModalOpen,
    compoundClipNameDraft,
    setCompoundClipNameDraft,
    speedDurationOpen,
    setSpeedDurationOpen,
    gpuTelemetryOpen,
    setGpuTelemetryOpen,
    gpuMetrics,
    ensureDiscardIsOkay,
    openProject,
    loadProjectFromPath,
    createProjectFromWelcome,
    recoverProjectFromWelcome,
    createProject,
    saveProject,
    chooseExportDestination,
    exportProject,
    handleExportTimelineInterop,
    handleImportTimelineInterop,
    handleFetchGpuTelemetry,
    toggleFullscreen,
    invokeMenuCommand,
    switchMulticamAngle,
    syncMulticamWaveforms,
  };
}
