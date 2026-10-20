export interface GlassEnvironment {
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
  dmabufRendererDisabled: boolean;
  webkitgtkVersion: string;
}

export async function probeEnvironment(): Promise<GlassEnvironment | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<GlassEnvironment>("glass_environment");
  } catch {
    return null;
  }
}

export async function saveReport(label: string, report: unknown): Promise<string | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<string>("save_glass_report", {
      label,
      report: JSON.stringify(report, null, 2),
    });
  } catch {
    return null;
  }
}

/**
 * Phase 0 mitigation: tag the document so CSS can fall back before the user
 * sees a frame. The class is a heuristic, not a benchmark result.
 */
export function tagGlassCapability(environment: GlassEnvironment | null) {
  const root = document.documentElement;
  const slow =
    !environment ||
    environment.gpuClass !== "discrete" ||
    environment.dmabufRendererDisabled;
  root.classList.toggle("webkit-gtk-slow", slow);
  root.dataset.webkitGtkSlow = String(slow);
  if (environment) {
    root.dataset.gpuClass = environment.gpuClass;
    root.dataset.gpuVendor = environment.gpuVendor;
  }
}
