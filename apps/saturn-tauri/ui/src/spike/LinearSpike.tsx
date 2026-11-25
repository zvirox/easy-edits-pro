// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useCallback, useEffect, useRef, useState } from "react";
import {
  probeEnvironment,
  saveReport,
  type GlassEnvironment,
} from "./environment";
import { measure, medianStats, type RunStats } from "./measure";
import "./linearSpike.css";

type Scene = "panels" | "video" | "empty" | "stress";

interface RunResult extends RunStats {
  videoFrames: number | null;
  videoFps: number | null;
}

interface ScenarioResult {
  name: string;
  scene: Scene;
  mode: "solid";
  drag: boolean;
  runs: RunResult[];
  median: RunStats;
  videoFps: number | null;
}

type FrameCallbackVideo = HTMLVideoElement & {
  requestVideoFrameCallback?: (callback: () => void) => number;
  cancelVideoFrameCallback?: (handle: number) => void;
};

function median(values: number[]): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0
    ? (sorted[middle - 1] + sorted[middle]) / 2
    : sorted[middle];
}

const REPEATS = 3;
const MEASURE_MS = 3500;
const WARMUP_MS = 800;
const STATIC_MS = 2500;
const AUTORUN = import.meta.env.VITE_GLASS_SPIKE_AUTORUN !== "0";
const LABEL =
  new URLSearchParams(window.location.search).get("label") ||
  import.meta.env.VITE_SPIKE_LABEL ||
  "run";

function nextFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function waitForVideo(video: HTMLVideoElement | null): Promise<boolean> {
  if (!video) return false;
  if (video.readyState >= 2) {
    try {
      await video.play();
      return !video.paused;
    } catch {
      return false;
    }
  }
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      cleanup();
      resolve(false);
    }, 8000);
    const cleanup = () => {
      window.clearTimeout(timer);
      video.removeEventListener("canplay", onReady);
    };
    const onReady = () => {
      cleanup();
      video
        .play()
        .then(() => resolve(!video.paused))
        .catch(() => resolve(false));
    };
    video.addEventListener("canplay", onReady);
  });
}

async function prepareWindow(): Promise<void> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const window_ = getCurrentWindow();
    await window_.setFullscreen(true);
    await window_.setFocus().catch(() => undefined);
    await new Promise((resolve) => window.setTimeout(resolve, 400));
  } catch {
    // Running outside Tauri
  }
}

export function LinearSpike() {
  const [scene, setScene] = useState<Scene>("panels");
  const [status, setStatus] = useState("probing");
  const [environment, setEnvironment] = useState<GlassEnvironment | null>(null);
  const [results, setResults] = useState<ScenarioResult[]>([]);
  const [reportPath, setReportPath] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const videoRef = useRef<HTMLVideoElement>(null);
  const dragRef = useRef<HTMLDivElement>(null);
  const drag2Ref = useRef<HTMLDivElement>(null);
  const liveRef = useRef<HTMLSpanElement>(null);
  const generation = useRef(0);

  const runScenario = useCallback(
    async (
      runId: number,
      name: string,
      nextScene: Scene,
      drag: boolean,
      durationMs: number,
    ): Promise<ScenarioResult | null> => {
      if (generation.current !== runId) return null;
      setScene(nextScene);
      setStatus(`${name} (warmup)`);
      await nextFrame();
      await nextFrame();
      if (nextScene === "panels" || nextScene === "video" || nextScene === "stress") {
        await waitForVideo(videoRef.current);
      }

      const runs: RunResult[] = [];
      for (let i = 0; i < REPEATS; i += 1) {
        if (generation.current !== runId) return null;
        setStatus(`${name} (${i + 1}/${REPEATS})`);
        const video = videoRef.current as FrameCallbackVideo | null;
        let videoFrames = 0;
        let handle = 0;
        const countFrame = () => {
          videoFrames += 1;
          if (video?.requestVideoFrameCallback) {
            handle = video.requestVideoFrameCallback(countFrame);
          }
        };
        if (video?.requestVideoFrameCallback) {
          handle = video.requestVideoFrameCallback(countFrame);
        }

        const startMs = performance.now();
        const stat = await measure({
          durationMs,
          warmupMs: WARMUP_MS,
          onFrame: drag
            ? (now) => {
                const t = (now - startMs) * 0.002;
                if (dragRef.current) {
                  dragRef.current.style.transform = `translate3d(${160 + Math.cos(t) * 140}px, ${140 + Math.sin(t * 1.3) * 110}px, 0)`;
                }
                if (nextScene === "stress" && drag2Ref.current) {
                  drag2Ref.current.style.transform = `translate3d(${240 + Math.sin(t * 0.9) * 150}px, ${180 + Math.cos(t * 1.1) * 120}px, 0)`;
                }
              }
            : undefined,
        });
        if (video?.cancelVideoFrameCallback && handle) {
          video.cancelVideoFrameCallback(handle);
        }
        const elapsed = (durationMs + WARMUP_MS) / 1000;
        runs.push({
          ...stat,
          videoFrames: video?.requestVideoFrameCallback ? videoFrames : null,
          videoFps: video?.requestVideoFrameCallback ? videoFrames / elapsed : null,
        });
      }

      const vFps = runs.map((r) => r.videoFps).filter((v): v is number => v !== null);
      return {
        name,
        scene: nextScene,
        mode: "solid",
        drag,
        runs,
        median: medianStats(runs),
        videoFps: median(vFps),
      };
    },
    [],
  );

  const benchmark = useCallback(async () => {
    const runId = ++generation.current;
    setFailure(null);
    setResults([]);
    await prepareWindow();
    const env = await probeEnvironment();
    if (generation.current !== runId) return;
    setEnvironment(env);

    const scenarios: [string, Scene, boolean, number][] = [
      ["static-empty", "empty", false, STATIC_MS],
      ["static-video", "video", false, STATIC_MS],
      ["static-solid-panels", "panels", false, STATIC_MS],
      ["drag-solid", "panels", true, MEASURE_MS],
      ["stress-solid", "stress", true, MEASURE_MS],
    ];
    const collected: ScenarioResult[] = [];
    for (const [name, nextScene, drag, duration] of scenarios) {
      const result = await runScenario(runId, name, nextScene, drag, duration);
      if (!result) return;
      collected.push(result);
      setResults([...collected]);
    }
    try {
      const path = await saveReport(LABEL, { environment: env, scenarios: collected });
      setReportPath(path);
      window.__linearSpike = { report: collected, path };
      setStatus("done");
    } catch (err) {
      setFailure(String(err));
      setStatus("error");
    }
  }, [runScenario]);

  useEffect(() => {
    if (AUTORUN) void benchmark();
    return () => {
      generation.current += 1;
    };
  }, [benchmark]);

  useEffect(() => {
    let frame = 0;
    let last = performance.now();
    let accumulated = 0;
    let frames = 0;
    const tick = (now: number) => {
      accumulated += now - last;
      frames += 1;
      last = now;
      if (accumulated >= 400) {
        if (liveRef.current) {
          liveRef.current.textContent = ((frames * 1000) / accumulated).toFixed(1);
        }
        accumulated = 0;
        frames = 0;
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);

  const summary = results.filter((result) => result.drag);

  return (
    <main className="linear-spike" data-scene={scene} data-status={status}>
      <div className="spike-stage" aria-hidden="true" />
      <video
        ref={videoRef}
        className="spike-video"
        src="/glass-spike.mp4"
        muted
        loop
        playsInline
        autoPlay
      />

      <section className="spike-panel spike-l1">
        <h2>Media</h2>
        <p>Solid surface · 1px border</p>
      </section>
      <section className="spike-panel spike-l2">
        <h2>Inspector</h2>
        <p>Solid surface · 1px border</p>
      </section>
      <section className="spike-panel spike-l3">
        <h2>Effects</h2>
        <p>Solid surface · 1px border</p>
      </section>

      <div ref={dragRef} className="spike-panel spike-drag">
        <h2>Draggable panel</h2>
        <p>Solid matte layer movement benchmark.</p>
      </div>

      <div ref={drag2Ref} className="spike-panel spike-drag2">
        <h2>Second draggable</h2>
        <p>Stress scene: two moving solid surfaces.</p>
      </div>

      <aside className="spike-hud">
        <div className="spike-hud-title">
          Linear spike · <span ref={liveRef}>--</span> fps live
        </div>
        <div>status: {status}</div>
        <div>scene: {scene}</div>
        {environment && (
          <div>
            {environment.gpuVendor} {environment.gpuClass} · {environment.gpuDriver} · WebKitGTK{" "}
            {environment.webkitgtkVersion}
          </div>
        )}
        {summary.map((result) => (
          <div key={result.name} className="spike-result">
            {result.name}: <strong>{result.median.fps.toFixed(1)} fps</strong> · p95{" "}
            {result.median.p95.toFixed(1)}ms
          </div>
        ))}
        {failure && <div className="spike-error">{failure}</div>}
        {reportPath && <div className="spike-path">report: {reportPath}</div>}
        <button type="button" className="spike-button" onClick={() => void benchmark()}>
          Run benchmark
        </button>
      </aside>
    </main>
  );
}

declare global {
  interface Window {
    __linearSpike?: { report: unknown; path: string | null };
  }
}
