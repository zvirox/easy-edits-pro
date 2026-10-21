export type ThemeMode = "dark" | "light";
export type GlassMode = "full" | "lite" | "off";
export type TrackHeightPreference = "compact" | "normal" | "expanded";

export interface Preferences {
  theme: ThemeMode;
  glass: GlassMode;
  /** null follows the operating system setting. */
  reducedMotion: boolean | null;
  snap: boolean;
  trackHeight: TrackHeightPreference;
}

export const PREFERENCES_STORAGE_KEY = "easy-edits-pro.preferences.v1";

export const DEFAULT_PREFERENCES: Preferences = {
  theme: "dark",
  glass: "full",
  reducedMotion: null,
  snap: true,
  trackHeight: "normal",
};

function legacyTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem("eep-theme");
    if (saved === "light" || saved === "dark") return saved;
  } catch {}
  return "dark";
}

function isGlassMode(value: unknown): value is GlassMode {
  return value === "full" || value === "lite" || value === "off";
}

function isTrackHeight(value: unknown): value is TrackHeightPreference {
  return value === "compact" || value === "normal" || value === "expanded";
}

/** Glass mode chosen by the user, or null when the runtime should decide. */
export function storedGlassMode(): GlassMode | null {
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { glass?: unknown };
    return isGlassMode(parsed.glass) ? parsed.glass : null;
  } catch {
    return null;
  }
}

export function loadPreferences(): Preferences {
  let stored: Partial<Preferences> = {};
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (raw) stored = JSON.parse(raw) as Partial<Preferences>;
  } catch {
    stored = {};
  }
  return {
    theme:
      stored.theme === "light" || stored.theme === "dark" ? stored.theme : legacyTheme(),
    glass: (() => {
      if (isGlassMode(stored.glass)) return stored.glass;
      const runtime = document.documentElement.getAttribute("data-glass");
      return isGlassMode(runtime) ? runtime : "full";
    })(),
    reducedMotion:
      stored.reducedMotion === true || stored.reducedMotion === false
        ? stored.reducedMotion
        : null,
    snap: stored.snap !== false,
    trackHeight: isTrackHeight(stored.trackHeight) ? stored.trackHeight : "normal",
  };
}

export function savePreferences(preferences: Preferences): void {
  try {
    localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(preferences));
  } catch {}
}
