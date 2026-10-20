export interface GpuDevice {
  vendor: string;
  model: string;
  class: string;
  driver: string;
  pciSlot: string;
}

export interface RuntimeEnvironment {
  appVersion: string;
  osName: string;
  kernel: string;
  sessionType: string;
  cpuModel: string;
  ramMb: number;
  gpuVendor: string;
  gpuModel: string;
  gpuClass: string;
  gpuDriver: string;
  gpuDevices: GpuDevice[];
  dmabufRendererDisabled: boolean;
  dmabufPolicy: string;
  webkitgtkVersion: string;
}

export async function probeEnvironment(): Promise<RuntimeEnvironment | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<RuntimeEnvironment>("glass_environment");
  } catch {
    // Browser-only development and the mocked smoke test do not have the
    // backend command. The UI still needs to render, so fall back to lite.
    return null;
  }
}

/**
 * Phase 0 measured 58.9 fps with DMA-BUF on an Intel Iris Xe, so "integrated"
 * is not a slow signal by itself. A missing probe or a disabled DMA-BUF
 * renderer is: those are the two cases where the preview path collapsed.
 */
export function isSlowEnvironment(environment: RuntimeEnvironment | null): boolean {
  return (
    !environment ||
    environment.dmabufRendererDisabled ||
    environment.gpuClass === "unknown"
  );
}

/**
 * Build-time kill switch. `VITE_GLASS_ENABLED=0` (or `false`) turns glass off
 * completely for a distribution build. Anything else, including the default
 * undefined value, keeps the measured full-glass path.
 */
export function isGlassEnabled(value: string | undefined = import.meta.env?.VITE_GLASS_ENABLED): boolean {
  return value !== "0" && value !== "false";
}

export type GlassMode = "full" | "lite" | "off";

export function glassMode(
  environment: RuntimeEnvironment | null,
  enabled: boolean = isGlassEnabled(),
): GlassMode {
  if (!enabled) return "off";
  return isSlowEnvironment(environment) ? "lite" : "full";
}

export function capabilityAttributes(environment: RuntimeEnvironment | null): Record<string, string> {
  if (!environment) {
    return {
      "data-webkit-gtk-slow": "true",
      "data-runtime-probe": "unavailable",
    };
  }
  return {
    "data-webkit-gtk-slow": String(isSlowEnvironment(environment)),
    "data-runtime-probe": "ready",
    "data-gpu-class": environment.gpuClass,
    "data-gpu-vendor": environment.gpuVendor,
    "data-gpu-driver": environment.gpuDriver,
    "data-dmabuf": environment.dmabufRendererDisabled ? "disabled" : "enabled",
    "data-dmabuf-policy": environment.dmabufPolicy,
  };
}

/**
 * Tag the document before the first glass frame can be committed. Phase 1 does
 * not paint glass yet; Phase 2's `.glass` rules consume the same tokens and
 * therefore inherit the lite fallback from `[data-webkit-gtk-slow="true"]`.
 */
export function tagGlassCapability(
  environment: RuntimeEnvironment | null,
  root: HTMLElement = document.documentElement,
  enabled: boolean = isGlassEnabled(),
): void {
  for (const [name, value] of Object.entries(capabilityAttributes(environment))) {
    root.setAttribute(name, value);
  }
  const mode = glassMode(environment, enabled);
  root.setAttribute("data-glass", mode);
  root.classList.toggle("webkit-gtk-slow", mode !== "full");
}
