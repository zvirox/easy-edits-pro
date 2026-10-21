import { InteriorButton } from "./interior/interior-button";
export interface StatusBarProps {
  busy: boolean;
  notice: string | null;
  connection: string;
  onFetchGpuTelemetry: () => Promise<void>;
  onToggleGpuTelemetry: () => void;
}

export function StatusBar({
  busy,
  notice,
  connection,
  onFetchGpuTelemetry,
  onToggleGpuTelemetry,
}: StatusBarProps) {
  return (
    <footer className="status-bar glass" data-elev="1">
      <span>{busy ? "Working…" : notice || "Autosave recovery enabled · saves every 30 seconds"}</span>
      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
        <InteriorButton
          type="button"
          className="gpu-status-pill"
          onClick={() => {
            void onFetchGpuTelemetry();
            onToggleGpuTelemetry();
          }}
          title="Native wgpu GPU Preview Surface Telemetry"
        >
          <span className="gpu-status-dot" />
          <span>⚡ Native GPU Preview (60 FPS)</span>
        </InteriorButton>
        <span>{connection}</span>
      </div>
    </footer>
  );
}
