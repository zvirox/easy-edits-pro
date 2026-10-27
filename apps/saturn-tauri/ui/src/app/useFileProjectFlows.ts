import { invoke } from "@tauri-apps/api/core";
import { confirm, save } from "@tauri-apps/plugin-dialog";
import {
  INTEROP_EXPORTS,
  recoveryNotice,
  sanitizeName,
  syntheticKindOf,
} from "./fileFlowTable";

export { FILE_FLOW_COMMANDS, INTEROP_EXPORTS, isFileFlowCommand } from "./fileFlowTable";

export interface FileFlowOptions {
  project: { name: string } | null;
  projectPath: string | null;
  perform: (id: string, params?: unknown, msg?: string) => Promise<boolean>;
  setNotice: (msg: string) => void;
  refreshProject: () => Promise<{ document: { project: { name: string } }; path: string | null }>;
  rememberProject: (name: string, path: string) => void;
  getPlayhead: () => number;
  closeToWelcome: () => void;
  /** The shared open-project flow: revert routes through it so the editor
   * resets selection/playhead exactly like File > Open does. */
  loadProjectFromPath: (path: string) => Promise<void>;
}

const projectFilters = [{ name: "Easy Edits Pro project", extensions: ["saturn"] }];

/**
 * FC file-family flows that are app-level orchestration (dialogs, Tauri
 * recovery commands, shell reveal) rather than engine state: the engine stays
 * the owner of project state, these only drive it through existing commands.
 */
export function createFileProjectFlows(options: FileFlowOptions) {
  const { project, projectPath, perform, setNotice, getPlayhead, closeToWelcome, loadProjectFromPath } =
    options;

  const saveTo = async (copy: boolean) => {
    if (!project) return;
    let path = copy ? null : (projectPath ?? null);
    if (!path) {
      path = await save({
        title: copy ? "Save a copy of the project" : "Save Easy Edits Pro project",
        defaultPath: `${sanitizeName(project.name)}.saturn`,
        filters: projectFilters,
      });
    }
    if (!path) return;
    const ok = await perform(
      "file.save",
      { path, copy },
      copy ? "Project copy saved" : "Project saved",
    );
    if (ok) {
      // After a copy the editor keeps working on the original file.
      options.rememberProject(project.name, copy ? (projectPath ?? path) : path);
    }
  };

  const revealFlow = async () => {
    if (!projectPath) {
      setNotice("Save the project first to reveal it on disk");
      return;
    }
    try {
      await invoke("reveal_path", { path: projectPath });
      setNotice("Revealed in the file manager");
    } catch (error) {
      setNotice(String(error));
    }
  };

  const exportFrameFlow = async () => {
    const target = await save({
      title: "Export Frame as PNG",
      defaultPath: `${sanitizeName(project?.name ?? "frame")}-${getPlayhead()}.png`,
      filters: [{ name: "PNG image", extensions: ["png"] }],
    });
    if (!target) return;
    try {
      await invoke("export_frame", { outputPath: target, atTick: getPlayhead() });
      setNotice("Frame exported as PNG");
    } catch (error) {
      setNotice(String(error));
    }
  };

  const interopExportFlow = async (id: string) => {
    const format = INTEROP_EXPORTS[id];
    if (!format || !project) return;
    const target = await save({
      title: `Export ${format.label}`,
      defaultPath: `${sanitizeName(project.name)}.${format.extension}`,
      filters: [{ name: format.label, extensions: [format.extension] }],
    });
    if (!target) return;
    const ok = await perform(id, { path: target }, `Exported ${format.label}`);
    if (ok) options.rememberProject(project.name, projectPath ?? target);
  };

  /** File > New synthetic media: generate server-side, import as media. */
  const generateSyntheticFlow = async (id: string) => {
    const kind = syntheticKindOf(id);
    if (!kind || !project) return;
    let color: string | undefined;
    if (kind === "color_matte") {
      const picked = window.prompt("Color matte hex color", "#808080");
      if (!picked) return;
      color = picked;
    }
    try {
      const result = await invoke<{
        path: string;
        kind: string;
        width: number;
        height: number;
      }>("generate_synthetic_media", {
        kind,
        color: color ?? null,
        durationSecs: kind === "bars_and_tone" ? 5.0 : null,
      });
      const fileKind = kind === "bars_and_tone" ? "video" : "image";
      const name = `${sanitizeName(project.name)} ${kind
        .replace(/_/g, " ")
        .replace(/\b\w/g, (c) => c.toUpperCase())}`;
      await perform(
        "project.add_media",
        {
          name,
          path: result.path,
          kind: fileKind,
          duration: kind === "bars_and_tone" ? 5 * 1_000_000 : 1,
          width: result.width,
          height: result.height,
        },
        `${name} generated and imported`,
      );
    } catch (error) {
      setNotice(String(error));
    }
  };

  const autosaveFlow = async (id: string) => {    try {
      if (id === "project.autosave.now") {
        await invoke("autosave_once");
        setNotice("Auto save written");
      } else if (id === "project.autosave.status") {
        const status = await invoke<Parameters<typeof recoveryNotice>[0]>("recovery_status");
        setNotice(recoveryNotice(status));
      } else if (id === "project.recovery.restore") {
        const proceed = await confirm(
          "Restore the autosaved project? Unsaved changes in the current session are lost.",
          { title: "Recover unsaved changes", kind: "warning" },
        );
        if (!proceed) return;
        await invoke("restore_autosave");
        const status = await options.refreshProject();
        if (status.path) options.rememberProject(status.document.project.name, status.path);
        setNotice("Autosaved project recovered. Save it to keep your changes.");
      } else if (id === "project.recovery.discard") {
        const proceed = await confirm("Delete the saved recovery snapshot?", {
          title: "Discard unsaved changes",
          kind: "warning",
        });
        if (!proceed) return;
        await invoke("discard_autosave");
        setNotice("Recovery snapshot discarded");
      }
    } catch (error) {
      setNotice(String(error));
    }
  };

  /** Single entry point used by invokeMenuCommand; returns false when the
   * id is not a file-flow command. */
  const handleFileCommand = async (id: string): Promise<boolean> => {
    switch (id) {
      case "file.saveAs":
      case "file.saveAll":
        await saveTo(false);
        return true;
      case "file.saveCopy":
        await saveTo(true);
        return true;
      case "file.revert": {
        if (!projectPath) {
          setNotice("The project has no saved file to revert to");
          return true;
        }
        const proceed = await confirm(
          "Revert to the last saved version? Unsaved changes are lost.",
          { title: "Revert project", kind: "warning" },
        );
        if (!proceed) return true;
        // Route through the shared open flow instead of a bare file.open:
        // reverting replaces the document, so editor-held selection and the
        // playhead must reset the same way File > Open resets them.
        await loadProjectFromPath(projectPath);
        setNotice(`Reverted to ${project?.name ?? "the saved project"}`);
        return true;
      }
      case "file.close":
      case "file.closeProject": {
        const proceed = await confirm("Close the current project? Unsaved changes are lost.", {
          title: "Close project",
          kind: "warning",
        });
        if (!proceed) return true;
        await perform("project.new", {}, "Project closed");
        closeToWelcome();
        return true;
      }
      case "file.reveal":
        await revealFlow();
        return true;
      case "export.frame":
        await exportFrameFlow();
        return true;
      default:
        if (INTEROP_EXPORTS[id]) {
          await interopExportFlow(id);
          return true;
        }
        if (syntheticKindOf(id)) {
          await generateSyntheticFlow(id);
          return true;
        }
        if (id.startsWith("project.autosave.") || id.startsWith("project.recovery.")) {
          await autosaveFlow(id);
          return true;
        }
        return false;
    }
  };

  return {
    handleFileCommand,
    saveTo,
    interopExportFlow,
    exportFrameFlow,
    generateSyntheticFlow,
    autosaveFlow,
  };
}
