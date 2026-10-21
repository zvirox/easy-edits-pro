import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { probeEnvironment, tagGlassCapability } from "./app/runtimeEnvironment";
import "./style.css";

// Tag the document before glass is painted so WebKitGTK machines with DMA-BUF
// disabled fall back to the lite token set.
void probeEnvironment().then(tagGlassCapability);

// Phase 0 feasibility spike. `VITE_GLASS_SPIKE=1 pnpm tauri dev` mounts the
// benchmark instead of the editor. The dynamic import keeps the spike out of
// production bundles: the flag is inlined at build time, so this branch is
// eliminated when it is not "1".
const root = createRoot(document.getElementById("root")!);

if (import.meta.env.VITE_GLASS_SPIKE === "1") {
  void import("./spike/GlassSpike").then(({ GlassSpike }) => {
    root.render(<GlassSpike />);
  });
} else {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
