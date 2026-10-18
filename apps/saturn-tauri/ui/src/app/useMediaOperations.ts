import {
  type Dispatch,
  type SetStateAction,
  useCallback,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { formatTime } from "../utils/formatters";
import type {
  Clip,
  ProjectDocument,
  ProjectStatus,
  TitleProperties,
  Transition,
  TransitionKind,
} from "./projectTypes";
import type { CommandStatus, MediaAvailability } from "./ProjectProvider";
import type { SelectedClip } from "../features/menubar";
import type { FilmoraTab, OpenverseAsset } from "../features/media-pool";
import { TICKS_PER_SECOND } from "../features/timeline/types";

const mediaFilters = [
  {
    name: "Video, audio, and images",
    extensions: [
      "mp4", "mkv", "mov", "webm", "avi", "m4v", "mp3", "wav", "flac", "ogg",
      "m4a", "aac", "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff",
    ],
  },
];

export interface UseMediaOperationsOptions {
  project: ProjectDocument["project"] | null;
  setProjectState: (status: ProjectStatus) => void;
  setMissingMediaIds: (updater: Set<number> | ((prev: Set<number>) => Set<number>)) => void;
  setUndoStatus: (status: CommandStatus) => void;
  setRedoStatus: (status: CommandStatus) => void;
  playhead: number;
  setPlayhead: (p: number) => void;
  refreshProject: () => Promise<unknown>;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  setNotice: (msg: string) => void;
  setBusy: (busy: boolean) => void;
  busy: boolean;
  selectedClip: SelectedClip | null;
  setSelectedClip: Dispatch<SetStateAction<SelectedClip | null>>;
  setSelectedClips: Dispatch<SetStateAction<SelectedClip[]>>;
  selectedTransition: { track_id: number; transition: Transition } | null;
  setSelectedTransition: Dispatch<SetStateAction<{ track_id: number; transition: Transition } | null>>;
  selectedTimelineClip: Clip | null;
  setWorkspace: (ws: any) => void;
}

export function useMediaOperations({
  project,
  setProjectState,
  setMissingMediaIds,
  setUndoStatus,
  setRedoStatus,
  playhead,
  setPlayhead,
  refreshProject,
  perform,
  runCommand,
  setNotice,
  setBusy,
  busy: _busy,
  selectedClip,
  setSelectedClip,
  setSelectedClips,
  selectedTransition,
  setSelectedTransition,
  selectedTimelineClip,
  setWorkspace,
}: UseMediaOperationsOptions) {
  const [selectedMediaId, setSelectedMediaId] = useState<number | null>(null);
  const [filmoraTab, setFilmoraTab] = useState<FilmoraTab>("media");
  const [search, setSearch] = useState("");

  const [mediaEditPropsId, setMediaEditPropsId] = useState<number | null>(null);
  const [mediaEditPropsName, setMediaEditPropsName] = useState("");
  const [mediaEditPropsColor, setMediaEditPropsColor] = useState("");
  const [mediaEditPropsTags, setMediaEditPropsTags] = useState("");

  const [newBinName, setNewBinName] = useState("");
  const [newBinColor, setNewBinColor] = useState("#38bdf8");
  const [createBinModalOpen, setCreateBinModalOpen] = useState(false);

  const [networkImportModalOpen, setNetworkImportModalOpen] = useState(false);
  const [batchQueueDrawerOpen, setBatchQueueDrawerOpen] = useState(false);
  const [aiChatDrawerOpen, setAiChatDrawerOpen] = useState(false);

  const [assetQuery, setAssetQuery] = useState("");
  const [assetType, setAssetType] = useState<"images" | "audio">("images");
  const [assetResults, setAssetResults] = useState<OpenverseAsset[]>([]);

  const [aiModel, setAiModel] = useState("llama3.2");
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiReply, setAiReply] = useState("");

  const [titleDraft, setTitleDraft] = useState<TitleProperties | null>(null);

  const importMedia = useCallback(async () => {
    const choice = await open({ title: "Import media into Easy Edits Pro", multiple: true, filters: mediaFilters });
    const paths = choice === null ? [] : Array.isArray(choice) ? choice : [choice];
    if (paths.length === 0) return;
    setBusy(true);
    setNotice(`Scanning ${paths.length} file${paths.length === 1 ? "" : "s"}…`);
    try {
      const status = await invoke<ProjectStatus>("import_media", { paths });
      setProjectState(status);
      const availability = await invoke<MediaAvailability[]>("check_media_availability");
      setMissingMediaIds(new Set(availability.filter((item) => item.missing).map((item) => item.mediaId)));
      const lastItem = status.document.project.media.at(-1);
      if (lastItem) setSelectedMediaId(lastItem.id);
      setSelectedClip(null);
      setSelectedClips([]);
      setNotice(`${paths.length} media file${paths.length === 1 ? "" : "s"} imported`);
      const [undo, redo] = await Promise.all([
        invoke<CommandStatus>("command_status", { id: "edit.undo" }),
        invoke<CommandStatus>("command_status", { id: "edit.redo" }),
      ]);
      setUndoStatus(undo);
      setRedoStatus(redo);
    } catch (error) {
      setNotice(`Import failed: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }, [setBusy, setNotice, setProjectState, setMissingMediaIds, setSelectedClip, setSelectedClips, setUndoStatus, setRedoStatus]);

  const relinkMedia = useCallback(
    async (mediaId: number) => {
      const path = await open({ title: "Relink missing media", multiple: false, filters: mediaFilters });
      if (typeof path !== "string") return;
      const ok = await perform("project.relink_media", { media_id: mediaId, path }, "Media relinked");
      if (ok) {
        setMissingMediaIds((current) => {
          const next = new Set(current);
          next.delete(mediaId);
          return next;
        });
      }
    },
    [perform, setMissingMediaIds]
  );

  const createMediaBin = useCallback(
    async (name: string, color?: string) => {
      if (!name.trim()) return;
      await perform("bin.create", { name: name.trim(), color: color || "#38bdf8" }, `Media bin "${name}" created`);
      setCreateBinModalOpen(false);
      setNewBinName("");
    },
    [perform]
  );

  const deleteMediaBin = useCallback(
    async (binId: number) => {
      await perform("bin.delete", { bin_id: binId }, "Media bin deleted");
    },
    [perform]
  );

  const moveMediaToBin = useCallback(
    async (mediaId: number, binId: number | null) => {
      await perform("bin.move_media", { media_id: mediaId, bin_id: binId }, "Moved media to bin");
    },
    [perform]
  );

  const updateMediaProperties = useCallback(
    async (mediaId: number, name?: string, colorLabel?: string, tags?: string[]) => {
      await perform("media.update_properties", {
        media_id: mediaId,
        name: name || undefined,
        color_label: colorLabel || undefined,
        tags: tags || undefined,
      }, "Asset properties updated");
      setMediaEditPropsId(null);
    },
    [perform]
  );

  const searchAssets = useCallback(async () => {
    setBusy(true);
    setNotice("");
    try {
      const results = await invoke<OpenverseAsset[]>("search_openverse", { query: assetQuery, mediaType: assetType });
      setAssetResults(results);
      setNotice(`${results.length} openly licensed result${results.length === 1 ? "" : "s"} from Openverse`);
    } catch (error) {
      setNotice(String(error));
      throw error;
    } finally {
      setBusy(false);
    }
  }, [assetQuery, assetType, setBusy, setNotice]);

  const addLibraryAsset = useCallback(
    async (asset: OpenverseAsset) => {
      setBusy(true);
      setNotice(`Downloading ${asset.title}…`);
      try {
        const path = await invoke<string>("download_openverse_asset", { asset });
        const status = await invoke<ProjectStatus>("import_media", { paths: [path] });
        setProjectState(status);
        setSelectedMediaId(status.document.project.media.at(-1)?.id ?? null);
        setWorkspace("edit");
        setNotice(`${asset.title} added. Attribution was saved beside the downloaded asset.`);
      } catch (error) {
        setNotice(`Could not add asset: ${String(error)}`);
      } finally {
        setBusy(false);
      }
    },
    [setBusy, setNotice, setProjectState, setWorkspace]
  );

  const askAi = useCallback(async () => {
    setBusy(true);
    setNotice("Waiting for the local Ollama model…");
    try {
      const response = await invoke<string>("ask_local_ai", { prompt: aiPrompt, model: aiModel });
      setAiReply(response);
      setNotice("Response received from local Ollama");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [aiPrompt, aiModel, setBusy, setNotice]);

  const addSelectedToTimeline = useCallback(async () => {
    const selectedMedia = project?.media.find((m) => m.id === selectedMediaId);
    if (!selectedMedia || !project) return;
    const desiredKind = selectedMedia.kind === "audio" ? "audio" : "video";
    const track = project.sequence.tracks.find((item) => item.kind === desiredKind);
    if (!track) {
      setNotice(`This project has no ${desiredKind} track`);
      return;
    }
    setBusy(true);
    try {
      const result = await runCommand<SelectedClip>("timeline.add_clip", { media_id: selectedMedia.id, track_id: track.id });
      setSelectedClip(result);
      setSelectedClips([]);
      setSelectedTransition(null);
      setPlayhead(0);
      await refreshProject();
      setNotice("Clip added to timeline");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [project, selectedMediaId, setNotice, setBusy, runCommand, setSelectedClip, setSelectedClips, setSelectedTransition, setPlayhead, refreshProject]);

  const addTitleToTimeline = useCallback(async () => {
    if (!project) return;
    const videoTrack = project.sequence.tracks.find((item) => item.kind === "video");
    if (!videoTrack) {
      setNotice("This project has no video track");
      return;
    }
    setBusy(true);
    setNotice("");
    try {
      const result = await runCommand<{ media_id: number; track_id: number; clip_index: number }>("timeline.add_title", {
        text: "Title Text",
        subtitle: "Subtitle / Description",
        style: "lower_third",
        timeline_start: playhead,
        track_id: videoTrack.id,
      });
      await refreshProject();
      setSelectedClip({ track_id: result.track_id, clip_index: result.clip_index });
      setSelectedClips([]);
      setSelectedTransition(null);
      setNotice("Title graphic added to timeline");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [project, playhead, setNotice, setBusy, runCommand, refreshProject, setSelectedClip, setSelectedClips, setSelectedTransition]);

  const saveTitleProperty = useCallback(
    async (props: Partial<TitleProperties>) => {
      if (!selectedTimelineClip || !titleDraft) return;
      const next = { ...titleDraft, ...props };
      setTitleDraft(next);
      await perform("title.update", {
        media_id: selectedTimelineClip.media_id,
        ...props,
      }, "Title graphic updated");
    },
    [selectedTimelineClip, titleDraft, perform]
  );

  const addPresetTitle = useCallback(
    async (preset: { name: string; style: string; text: string; subtitle: string }) => {
      if (!project) return;
      const videoTrack = project.sequence.tracks.find((item) => item.kind === "video");
      if (!videoTrack) {
        setNotice("This project has no video track");
        return;
      }
      setBusy(true);
      setNotice("");
      try {
        const result = await runCommand<{ media_id: number; track_id: number; clip_index: number }>("timeline.add_title", {
          text: preset.text,
          subtitle: preset.subtitle,
          style: preset.style,
          timeline_start: playhead,
          track_id: videoTrack.id,
        });
        await refreshProject();
        setSelectedClip({ track_id: result.track_id, clip_index: result.clip_index });
        setSelectedClips([]);
        setSelectedTransition(null);
        setNotice(`Added ${preset.name} at ${formatTime(playhead)}`);
      } catch (error) {
        setNotice(String(error));
      } finally {
        setBusy(false);
      }
    },
    [project, playhead, setNotice, setBusy, runCommand, refreshProject, setSelectedClip, setSelectedClips, setSelectedTransition]
  );

  const addPresetTransition = useCallback(
    async (kind: TransitionKind) => {
      if (!project) return;
      let targetTrackId = selectedClip?.track_id;
      let cutAt = playhead;
      if (selectedClip && selectedTimelineClip) {
        targetTrackId = selectedClip.track_id;
        const distStart = Math.abs(playhead - selectedTimelineClip.timeline_start);
        const distEnd = Math.abs(playhead - (selectedTimelineClip.timeline_start + selectedTimelineClip.duration));
        cutAt = distStart < distEnd ? selectedTimelineClip.timeline_start : selectedTimelineClip.timeline_start + selectedTimelineClip.duration;
      } else {
        const videoTrack = project.sequence.tracks.find((t) => t.kind === "video");
        if (videoTrack) targetTrackId = videoTrack.id;
      }
      if (targetTrackId == null) {
        setNotice("Select a video clip or track to apply transition");
        return;
      }
      setBusy(true);
      setNotice("");
      try {
        const res = await runCommand<{ transition_id: number; track_id: number; kind: string; cut_at: number; duration: number }>(
          "timeline.add_transition",
          {
            track_id: targetTrackId,
            cut_at: cutAt,
            duration: TICKS_PER_SECOND,
            kind,
          }
        );
        if (res.kind !== kind) {
          await runCommand("timeline.update_transition", {
            track_id: res.track_id,
            transition_id: res.transition_id,
            kind,
            duration: TICKS_PER_SECOND,
          });
        }
        await refreshProject();
        setSelectedTransition({
          track_id: res.track_id,
          transition: {
            id: res.transition_id,
            kind,
            cut_at: res.cut_at,
            duration: TICKS_PER_SECOND,
          },
        });
        setSelectedClip(null);
        setSelectedClips([]);
        setNotice(`Applied ${kind.replace(/_/g, " ")} transition at ${formatTime(cutAt)}`);
      } catch (error) {
        setNotice(String(error));
      } finally {
        setBusy(false);
      }
    },
    [project, selectedClip, playhead, selectedTimelineClip, setNotice, setBusy, runCommand, refreshProject, setSelectedTransition, setSelectedClip, setSelectedClips]
  );

  const addTransitionToTimeline = useCallback(async () => {
    return addPresetTransition("crossfade");
  }, [addPresetTransition]);

  const removeSelectedTransition = useCallback(async () => {
    if (!selectedTransition) return;
    setBusy(true);
    setNotice("");
    try {
      await runCommand("timeline.remove_transition", {
        track_id: selectedTransition.track_id,
        transition_id: selectedTransition.transition.id,
      });
      setSelectedTransition(null);
      await refreshProject();
      setNotice("Transition removed");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [selectedTransition, setBusy, setNotice, runCommand, setSelectedTransition, refreshProject]);

  const updateTransition = useCallback(
    async (kind: TransitionKind, duration: number) => {
      if (!selectedTransition) return;
      setBusy(true);
      setNotice("");
      try {
        const res = await runCommand<{ transition_id: number; track_id: number; kind: string; cut_at: number; duration: number }>(
          "timeline.add_transition",
          {
            track_id: selectedTransition.track_id,
            cut_at: selectedTransition.transition.cut_at,
            duration,
            kind,
          }
        );
        await refreshProject();
        setSelectedTransition({
          track_id: res.track_id,
          transition: {
            id: res.transition_id,
            kind: res.kind as TransitionKind,
            cut_at: res.cut_at,
            duration: res.duration,
          },
        });
        setNotice(`Transition updated to ${kind.replace(/_/g, " ")}`);
      } catch (error) {
        setNotice(String(error));
      } finally {
        setBusy(false);
      }
    },
    [selectedTransition, setBusy, setNotice, runCommand, refreshProject, setSelectedTransition]
  );

  return {
    selectedMediaId,
    setSelectedMediaId,
    filmoraTab,
    setFilmoraTab,
    search,
    setSearch,
    mediaEditPropsId,
    setMediaEditPropsId,
    mediaEditPropsName,
    setMediaEditPropsName,
    mediaEditPropsColor,
    setMediaEditPropsColor,
    mediaEditPropsTags,
    setMediaEditPropsTags,
    newBinName,
    setNewBinName,
    newBinColor,
    setNewBinColor,
    createBinModalOpen,
    setCreateBinModalOpen,
    networkImportModalOpen,
    setNetworkImportModalOpen,
    batchQueueDrawerOpen,
    setBatchQueueDrawerOpen,
    aiChatDrawerOpen,
    setAiChatDrawerOpen,
    assetQuery,
    setAssetQuery,
    assetType,
    setAssetType,
    assetResults,
    setAssetResults,
    aiModel,
    setAiModel,
    aiPrompt,
    setAiPrompt,
    aiReply,
    setAiReply,
    titleDraft,
    setTitleDraft,
    importMedia,
    relinkMedia,
    createMediaBin,
    deleteMediaBin,
    moveMediaToBin,
    updateMediaProperties,
    searchAssets,
    addLibraryAsset,
    askAi,
    addSelectedToTimeline,
    addTitleToTimeline,
    saveTitleProperty,
    addPresetTitle,
    addPresetTransition,
    addTransitionToTimeline,
    removeSelectedTransition,
    updateTransition,
  };
}
