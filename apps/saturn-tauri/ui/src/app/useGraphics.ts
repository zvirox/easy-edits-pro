import { useCallback, useEffect, useMemo, useState } from "react";
import type { GraphicItem, GraphicTemplate } from "../engine/playback/types";
import { PANEL_OPEN_EVENT } from "../features/panels/presets";
import { TICKS_PER_SECOND } from "../utils/formatters.ts";
import { creationBox } from "../features/graphics/graphicsOverlay";

export const GRAPHICS_PANEL_ID = "graphics";

export interface UseGraphicsOptions {
  project: {
    sequence: {
      graphics?: GraphicItem[];
    };
    graphic_templates?: GraphicTemplate[];
    settings?: { width: number; height: number };
  } | null;
  playhead: number;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
}

export interface GraphicOperations {
  graphics: GraphicItem[];
  templates: GraphicTemplate[];
  selectedId: number | null;
  selected: GraphicItem | null;
  setSelectedId: (id: number | null) => void;
  fonts: string[];
  /** Load (and cache) system font families for the typeface dropdown. */
  ensureFonts: () => Promise<void>;
  addGraphic: (kind: "text" | "rectangle" | "ellipse") => Promise<void>;
  removeGraphic: (id: number) => Promise<void>;
  duplicateGraphic: (id: number) => Promise<void>;
  /** Partial transform update; accepts any subset of the inspector fields. */
  updateTransform: (id: number, patch: Record<string, number>) => Promise<void>;
  updateText: (id: number, patch: Record<string, unknown>) => Promise<void>;
  setFill: (id: number, patch: { color?: string; opacity?: number }) => Promise<void>;
  setStroke: (id: number, patch: { color?: string; width?: number }) => Promise<void>;
  setShadow: (
    id: number,
    patch: { color?: string; blur?: number; offset_x?: number; offset_y?: number },
  ) => Promise<void>;
  /** Toggles visibility/lock (Phase 5 slice 4). */
  setFlags: (id: number, patch: { visible?: boolean; locked?: boolean }) => Promise<void>;
  /** Creates a graphic centred on a normalized canvas point (canvas tools). */
  createAt: (
    kind: "text" | "rectangle" | "ellipse",
    xNorm: number,
    yNorm: number,
    vertical?: boolean,
  ) => Promise<void>;
  alignSelection: (mode: string, ids: number[]) => Promise<void>;
  arrange: (id: number, order: "front" | "back" | "forward" | "backward") => Promise<void>;
  saveTemplate: (id: number) => Promise<void>;
  applyTemplate: (templateId: number) => Promise<void>;
  /** Handles the graphics commands that need UI gestures; false = not ours. */
  handleMenuCommand: (id: string) => Promise<boolean>;
  openPanel: () => void;
}

export function useGraphics({
  project,
  playhead,
  runCommand,
  refreshProject,
  setNotice,
}: UseGraphicsOptions): GraphicOperations {
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [fonts, setFonts] = useState<string[]>([]);

  const graphics = useMemo(
    () => project?.sequence.graphics ?? [],
    [project?.sequence.graphics],
  );
  const templates = useMemo(
    () => project?.graphic_templates ?? [],
    [project?.graphic_templates],
  );
  const selected = useMemo(
    () => graphics.find((graphic) => graphic.id === selectedId) ?? null,
    [graphics, selectedId],
  );

  // Drop the selection when the graphic disappears (undo, remove).
  useEffect(() => {
    if (selectedId !== null && !graphics.some((graphic) => graphic.id === selectedId)) {
      setSelectedId(null);
    }
  }, [graphics, selectedId]);

  const openPanel = useCallback(() => {
    const announce = () =>
      window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: GRAPHICS_PANEL_ID } }));
    announce();
    // Same-tick workspace switches subscribe a frame later; re-announce.
    window.setTimeout(announce, 120);
  }, []);

  const ensureFonts = useCallback(async () => {
    if (fonts.length > 0) return;
    try {
      const res = await runCommand<{ fonts: string[] }>("fonts.list");
      setFonts(res?.fonts ?? []);
    } catch (error) {
      setNotice(String(error));
    }
  }, [fonts.length, runCommand, setNotice]);

  const addGraphic = useCallback(
    async (kind: "text" | "rectangle" | "ellipse") => {
      try {
        const duration = 5 * TICKS_PER_SECOND;
        const res = await runCommand<{ id: number }>("graphic.add", {
          kind,
          start_ticks: playhead,
          duration_ticks: duration,
          ...(kind === "text" ? { text: "Your title" } : {}),
        });
        await refreshProject();
        if (res) setSelectedId(res.id);
        setNotice("Graphic added at playhead");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [playhead, runCommand, refreshProject, setNotice],
  );

  const removeGraphic = useCallback(
    async (id: number) => {
      try {
        await runCommand("graphic.remove", { id });
        await refreshProject();
        if (selectedId === id) setSelectedId(null);
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, selectedId, setNotice],
  );

  const duplicateGraphic = useCallback(
    async (id: number) => {
      try {
        const res = await runCommand<{ id: number }>("graphic.duplicate", { id });
        await refreshProject();
        if (res) setSelectedId(res.id);
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const updateTransform = useCallback(
    async (id: number, patch: Record<string, number>) => {
      try {
        await runCommand("graphic.set_transform", { id, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const updateText = useCallback(
    async (id: number, patch: Record<string, unknown>) => {
      try {
        await runCommand("graphic.set_text", { id, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const setFill = useCallback(
    async (id: number, patch: { color?: string; opacity?: number }) => {
      try {
        await runCommand("graphic.set_fill", { id, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const setStroke = useCallback(
    async (id: number, patch: { color?: string; width?: number }) => {
      try {
        await runCommand("graphic.set_stroke", { id, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const setShadow = useCallback(
    async (
      id: number,
      patch: { color?: string; blur?: number; offset_x?: number; offset_y?: number },
    ) => {
      try {
        await runCommand("graphic.set_shadow", { id, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const setFlags = useCallback(
    async (id: number, patch: { visible?: boolean; locked?: boolean }) => {
      try {
        await runCommand("graphic.set_flags", { id, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const createAt = useCallback(
    async (
      kind: "text" | "rectangle" | "ellipse",
      xNorm: number,
      yNorm: number,
      vertical = false,
    ) => {
      try {
        const frame = project?.settings ?? { width: 1920, height: 1080 };
        const box = creationBox(kind, xNorm, yNorm, frame);
        const res = await runCommand<{ id: number }>("graphic.add", {
          kind,
          start_ticks: playhead,
          duration_ticks: 5 * TICKS_PER_SECOND,
          ...(kind === "text" ? { text: vertical ? "縦書き" : "Your title" } : {}),
        });
        const id = res?.id;
        if (id) {
          await runCommand("graphic.set_transform", { id, x: box.x, y: box.y, width: box.width, height: box.height });
          if (vertical) {
            await runCommand("graphic.set_text", { id, vertical: true });
          }
          setSelectedId(id);
        }
        await refreshProject();
        setNotice("Graphic created");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [playhead, project?.settings, runCommand, refreshProject, setNotice],
  );

  const alignSelection = useCallback(
    async (mode: string, ids: number[]) => {
      if (ids.length < 2) {
        setNotice("Select at least two graphics to align");
        return;
      }
      try {
        await runCommand("graphic.align", { ids, mode });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const arrange = useCallback(
    async (id: number, order: "front" | "back" | "forward" | "backward") => {
      try {
        await runCommand("graphic.arrange", { id, order });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const saveTemplate = useCallback(
    async (id: number) => {
      const name = window.prompt("Template name", "My graphic");
      if (!name || !name.trim()) return;
      try {
        await runCommand("graphic.template_save", { graphic_id: id, name: name.trim() });
        await refreshProject();
        setNotice("Graphic template saved");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const applyTemplate = useCallback(
    async (templateId: number) => {
      try {
        const res = await runCommand<{ id: number }>("graphic.template_apply", {
          template_id: templateId,
          start_ticks: playhead,
        });
        await refreshProject();
        if (res) setSelectedId(res.id);
        setNotice("Template applied at playhead");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [playhead, runCommand, refreshProject, setNotice],
  );

  const handleMenuCommand = useCallback(
    async (id: string) => {
      if (id === "graphic.panel") {
        openPanel();
        return true;
      }
      return false;
    },
    [openPanel],
  );

  return {
    graphics,
    templates,
    selectedId,
    selected,
    setSelectedId,
    fonts,
    ensureFonts,
    addGraphic,
    removeGraphic,
    duplicateGraphic,
    updateTransform,
    updateText,
    setFill,
    setStroke,
    setShadow,
    setFlags,
    createAt,
    alignSelection,
    arrange,
    saveTemplate,
    applyTemplate,
    handleMenuCommand,
    openPanel,
  };
}
