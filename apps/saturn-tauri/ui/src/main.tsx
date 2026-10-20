import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { probeEnvironment, tagGlassCapability } from "./app/runtimeEnvironment";
import "./style.css";

// Phase 0's mitigation: tag the document before glass is painted so WebKitGTK
// machines with DMA-BUF disabled fall back to the lite token set.
void probeEnvironment().then(tagGlassCapability);

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
