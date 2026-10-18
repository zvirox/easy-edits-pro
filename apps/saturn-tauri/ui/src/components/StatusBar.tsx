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
    <footer className="status-bar">
      <span>{busy ? "Working…" : notice || "Autosave recovery enabled · saves every 30 seconds"}</span>
      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
        <button
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
        </button>
        <span>{connection}</span>
      </div>
    </footer>
  );
}
