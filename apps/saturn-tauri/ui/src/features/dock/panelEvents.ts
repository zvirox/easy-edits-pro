/**
 * Panel gesture events (G2, plan v4 §4).
 *
 * The MDI floating-panel engine is gone; these window events now drive
 * dock-card tab activation (features/dock/DockCard.tsx). The event contract —
 * names and detail shapes — is unchanged from features/panels/presets.ts, so
 * app hooks and menu commands keep working across the move.
 */
export const PANEL_OPEN_EVENT = "eep:panel-open";
export const PANEL_TOGGLE_EVENT = "eep:panel-toggle";

/** Detail payload carried by both panel events. */
export interface PanelOpenDetail {
  id: string;
}
