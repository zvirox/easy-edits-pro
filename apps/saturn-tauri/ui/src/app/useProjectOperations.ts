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
  ProjectStatus,
  SelectedClip,
  SilenceInterval,
  Transition,
} from "./projectTypes";

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
}: UseProjectOperationsOptions) {
  const project = projectState?.document.project ?? null;

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

  // Transcribe modal
  const [transcribeModalOpen, setTranscribeModalOpen] = useState(false);
  const [transcribeLanguage, setTranscribeLanguage] = useState("en");
  const [transcribeModel, setTranscribeModel] = useState("base");
  const [transcribeTextOverride, setTranscribeTextOverride] = useState("");
  const [transcribeReplaceExisting, setTranscribeReplaceExisting] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);

  // Silence removal modal
  const [silenceModalOpen, setSilenceModalOpen] = useState(false);
  const [silenceThresholdDb, setSilenceThresholdDb] = useState(-35);
  const [silenceMinDurationMs, setSilenceMinDurationMs] = useState(400);
  const [silencePaddingMs, setSilencePaddingMs] = useState(50);
  const [silenceIntervals, setSilenceIntervals] = useState<SilenceInterval[]>([]);
  const [isDetectingSilence, setIsDetectingSilence] = useState(false);

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

  // GPU Telemetry
  const [gpuTelemetryOpen, setGpuTelemetryOpen] = useState(false);
  const [gpuMetrics, setGpuMetrics] = useState<GpuMetrics | null>(null);

  // Collab
  const [collabModalOpen, setCollabModalOpen] = useState(false);
  const [collabUser] = useState(() => {
    try {
      const saved = localStorage.getItem("eep-collab-user");
      if (saved) return JSON.parse(saved);
    } catch {}
    return { id: `user-${Math.floor(Math.random() * 9000 + 1000)}`, name: "Lead Editor", color: "#38bdf8" };
  });

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

  const exportProject = useCallback(async () => {
    if (!project || rendering) return;
    const path = await save({
      title: "Export video",
      defaultPath: `${project.name.replace(/[\\/:*?"<>|]/g, "-")}.${exportFormat}`,
      filters: [{ name: exportFormat === "mp4" ? "MP4 video" : "WebM video", extensions: [exportFormat] }],
    });
    if (!path) return;

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
          width: project.settings.width,
          height: project.settings.height,
          frameRateNumerator: project.settings.frame_rate.numerator,
          frameRateDenominator: project.settings.frame_rate.denominator,
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
  }, [project, rendering, exportFormat, exportMarkedRange, useHardwareEncoding, setNotice]);

  const handleAutoTranscribe = useCallback(async () => {
    setIsTranscribing(true);
    setNotice("Transcribing timeline audio with Whisper AI…");
    try {
      await invoke("transcribe_timeline_speech", {
        model: transcribeModel,
        language: transcribeLanguage === "auto" ? null : transcribeLanguage,
        textOverride: transcribeTextOverride || null,
        replaceExisting: transcribeReplaceExisting,
      });
      await refreshProject();
      setTranscribeModalOpen(false);
      setNotice("Auto-transcription complete! Subtitle cues added.");
    } catch (err) {
      setNotice(`Transcription failed: ${String(err)}`);
    } finally {
      setIsTranscribing(false);
    }
  }, [transcribeModel, transcribeLanguage, transcribeTextOverride, transcribeReplaceExisting, setNotice, refreshProject]);

  const handleDetectSilence = useCallback(async () => {
    setIsDetectingSilence(true);
    setNotice("Analyzing audio waveform for silence gaps…");
    try {
      const intervals = await invoke<SilenceInterval[]>("detect_silence_intervals", {
        thresholdDb: silenceThresholdDb,
        minDurationMs: silenceMinDurationMs,
      });
      setSilenceIntervals(intervals);
      setNotice(`Detected ${intervals.length} silence intervals across timeline.`);
    } catch (err) {
      setNotice(`Silence detection failed: ${String(err)}`);
    } finally {
      setIsDetectingSilence(false);
    }
  }, [silenceThresholdDb, silenceMinDurationMs, setNotice]);

  const handleRippleDeleteSilence = useCallback(async () => {
    setBusy(true);
    setNotice("Ripple-deleting silence gaps…");
    try {
      await invoke("ripple_delete_silence_intervals", {
        thresholdDb: silenceThresholdDb,
        minDurationMs: silenceMinDurationMs,
        paddingMs: silencePaddingMs,
      });
      await refreshProject();
      setSilenceModalOpen(false);
      setNotice("Successfully ripple-removed silence pauses from timeline!");
    } catch (err) {
      setNotice(`Ripple delete failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }, [silenceThresholdDb, silenceMinDurationMs, silencePaddingMs, setBusy, setNotice, refreshProject]);

  const handleRippleDeleteFillers = useCallback(async () => {
    setBusy(true);
    setNotice("Ripple-deleting verbal hesitation fillers…");
    try {
      await invoke("ripple_delete_filler_words", {
        paddingMs: silencePaddingMs,
      });
      await refreshProject();
      setSilenceModalOpen(false);
      setNotice("Successfully removed filler words from timeline!");
    } catch (err) {
      setNotice(`Filler removal failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }, [silencePaddingMs, setBusy, setNotice, refreshProject]);

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

  const invokeMenuCommand = useCallback(
    async (id: string) => {
      switch (id) {
        case "file.new":
          setNewProjectOpen(true);
          break;
        case "file.open":
          await openProject();
          break;
        case "file.save":
          await saveProject();
          break;
        case "view.fullscreen":
          await toggleFullscreen();
          break;
        default:
          await perform(id);
      }
    },
    [openProject, saveProject, toggleFullscreen, perform]
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
    transcribeModalOpen,
    setTranscribeModalOpen,
    transcribeLanguage,
    setTranscribeLanguage,
    transcribeModel,
    setTranscribeModel,
    transcribeTextOverride,
    setTranscribeTextOverride,
    transcribeReplaceExisting,
    setTranscribeReplaceExisting,
    isTranscribing,
    silenceModalOpen,
    setSilenceModalOpen,
    silenceThresholdDb,
    setSilenceThresholdDb,
    silenceMinDurationMs,
    setSilenceMinDurationMs,
    silencePaddingMs,
    setSilencePaddingMs,
    silenceIntervals,
    isDetectingSilence,
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
    gpuTelemetryOpen,
    setGpuTelemetryOpen,
    gpuMetrics,
    collabModalOpen,
    setCollabModalOpen,
    collabUser,
    ensureDiscardIsOkay,
    openProject,
    loadProjectFromPath,
    createProjectFromWelcome,
    recoverProjectFromWelcome,
    createProject,
    saveProject,
    exportProject,
    handleAutoTranscribe,
    handleDetectSilence,
    handleRippleDeleteSilence,
    handleRippleDeleteFillers,
    handleExportTimelineInterop,
    handleImportTimelineInterop,
    handleFetchGpuTelemetry,
    toggleFullscreen,
    invokeMenuCommand,
  };
}
