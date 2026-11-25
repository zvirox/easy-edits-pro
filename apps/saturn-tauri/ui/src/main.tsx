// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { WebDeskShell } from "./features/webdesk";
import "./style.css";

// Solid Linear UI is final and locked; enforce data-glass="off" on root.
document.documentElement.setAttribute("data-glass", "off");

const isWebDesk =
  typeof window !== "undefined" &&
  (new URLSearchParams(window.location.search).get("mode") === "desk" ||
    new URLSearchParams(window.location.search).get("desk") === "1");

const root = createRoot(document.getElementById("root")!);

if (import.meta.env.VITE_LINEAR_SPIKE === "1" || import.meta.env.VITE_GLASS_SPIKE === "1") {
  void import("./spike/LinearSpike").then(({ LinearSpike }) => {
    root.render(<LinearSpike />);
  });
} else if (isWebDesk) {
  root.render(
    <React.StrictMode>
      <WebDeskShell />
    </React.StrictMode>,
  );
} else {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
