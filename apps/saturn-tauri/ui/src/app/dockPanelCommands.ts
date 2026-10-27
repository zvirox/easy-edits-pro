export interface DockPanelOps {
  // The helper only ever selects the Edit workspace.
  selectEditorWorkspace: (ws: "edit") => void;
  openMarkerPanel: () => void | Promise<void>;
  openCaptionPanel: () => void | Promise<void>;
  openGraphicPanel: () => void | Promise<void>;
}

/** Docked-adjacent panels (markers, captions, graphics) live in the Edit
 * workspace; opening one switches there and announces the panel. Handles the
 * three *.panel ui_only commands; returns false for anything else. */
export function openDockPanelCommand(id: string, ops: DockPanelOps): boolean {
  if (id !== "marker.panel" && id !== "caption.panel" && id !== "graphic.panel") {
    return false;
  }
  ops.selectEditorWorkspace("edit");
  if (id === "marker.panel") void ops.openMarkerPanel();
  else if (id === "caption.panel") void ops.openCaptionPanel();
  else void ops.openGraphicPanel();
  return true;
}
