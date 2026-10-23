import type { Preferences } from "../features/preferences/preferences";
import { PANEL_TOGGLE_EVENT } from "../features/dock/panelEvents";

export interface ViewCommandContext {
  preferences: Preferences;
  setPreferences: (update: (current: Preferences) => Preferences) => void;
  setPreferencesOpen: (open: boolean) => void;
  setShortcutsOpen: (open: boolean) => void;
  setNotice: (message: string) => void;
  setSnapEnabled: (update: (value: boolean) => boolean) => void;
  zoomTimeline: (factor: number) => void;
  setTimelineZoom: (zoom: number) => void;
}

/**
 * UI-only view and help commands, extracted from App's dispatch so the
 * workspace dispatcher stays under the source-size budget. Returns true when
 * the command was handled here.
 */
export function applyViewCommand(id: string, ctx: ViewCommandContext): boolean {
  if (id.startsWith("view.glass_")) {
    const glass = id.slice("view.glass_".length) as Preferences["glass"];
    ctx.setPreferences((current) => ({ ...current, glass }));
    return true;
  }
  switch (id) {
    case "view.toggle_snap":
      ctx.setSnapEnabled((value) => !value);
      return true;
    case "view.zoom_in":
      ctx.zoomTimeline(1.25);
      return true;
    case "view.zoom_out":
      ctx.zoomTimeline(0.8);
      return true;
    case "view.zoom_reset":
      ctx.setTimelineZoom(72);
      return true;
    case "view.theme_dark":
    case "view.theme_light":
      ctx.setPreferences((current) => ({
        ...current,
        theme: id === "view.theme_dark" ? "dark" : "light",
      }));
      return true;
    case "view.reduced_motion":
      ctx.setPreferences((current) => ({
        ...current,
        reducedMotion:
          current.reducedMotion === null ? true : current.reducedMotion ? false : null,
      }));
      return true;
    case "view.panel_media":
    case "view.panel_inspector":
      window.dispatchEvent(
        new CustomEvent(PANEL_TOGGLE_EVENT, {
          detail: { id: id === "view.panel_media" ? "media" : "inspector" },
        }),
      );
      return true;
    case "help.diagnostics":
      ctx.setNotice("Diagnostics are available from the status bar.");
      return true;
    case "help.support":
      ctx.setNotice("Report issues from the project repository.");
      return true;
    case "help.about":
      ctx.setNotice("Easy Edits Pro — glass NLE built on the Saturn engine.");
      return true;
    default:
      return false;
  }
}
