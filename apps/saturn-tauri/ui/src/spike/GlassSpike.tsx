import { useCallback, useEffect, useRef, useState } from "react";
import {
  probeEnvironment,
  saveReport,
  tagGlassCapability,
  type GlassEnvironment,
} from "./environment";
import { measure, medianStats, type RunStats } from "./measure";
import "./glassSpike.css";

type GlassMode = "full" | "lite" | "off";
type Scene = "panels" | "video" | "empty" | "stress";

interface RunResult extends RunStats {
  /** Frames the compositor actually presented, sampled with requestVideoFrameCallback. */
  videoFrames: number | null;
  videoFps: number | null;
}

interface ScenarioResult {
  name: string;
  scene: Scene;
  mode: GlassMode;
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

/**
 * A tiled compositor may open the window off-screen or unfocused, and WebKit
 * then throttles animation frames to a few per second. Fullscreen guarantees
 * the benchmark window is actually presented and composited.
 */
async function prepareWindow(): Promise<void> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const window_ = getCurrentWindow();
    await window_.setFullscreen(true);
    await window_.setFocus().catch(() => undefined);
    await new Promise((resolve) => window.setTimeout(resolve, 400));
  } catch {
    // Not running inside Tauri (or permission missing): keep the window as-is.
  }
}

export function GlassSpike() {
  const [scene, setScene] = useState<Scene>("panels");
  const [mode, setMode] = useState<GlassMode>("full");
  const [status, setStatus] = useState("probing");
  const [environment, setEnvironment] = useState<GlassEnvironment | null>(null);
  const [results, setResults] = useState<ScenarioResult[]>([]);
  const [reportPath, setReportPath] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const videoRef = useRef<HTMLVideoElement>(null);
  const dragRef = useRef<HTMLDivElement>(null);
  const drag2Ref = useRef<HTMLDivElement>(null);
  const liveRef = useRef<HTMLSpanElement>(null);
  const environmentRef = useRef<GlassEnvironment | null>(null);
  const generation = useRef(0);
  const busy = useRef(false);

  const moveDragPanel = useCallback((elapsedMs: number) => {
    const panel = dragRef.current;
    if (!panel) return;
    const maxX = Math.max(0, window.innerWidth - panel.offsetWidth - 24);
    const maxY = Math.max(0, window.innerHeight - panel.offsetHeight - 24);
    const x = 12 + ((Math.sin(elapsedMs / 620) + 1) / 2) * maxX;
    const y = 12 + ((Math.cos(elapsedMs / 880) + 1) / 2) * maxY;
    panel.style.transform = `translate3d(${x}px, ${y}px, 0)`;
  }, []);

  const moveSecondPanel = useCallback((elapsedMs: number) => {
    const panel = drag2Ref.current;
    if (!panel) return;
    const maxX = Math.max(0, window.innerWidth - panel.offsetWidth - 24);
    const maxY = Math.max(0, window.innerHeight - panel.offsetHeight - 24);
    const x = 12 + ((Math.cos(elapsedMs / 540) + 1) / 2) * maxX;
    const y = 12 + ((Math.sin(elapsedMs / 760) + 1) / 2) * maxY;
    panel.style.transform = `translate3d(${x}px, ${y}px, 0)`;
  }, []);

  const benchmark = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    const run = ++generation.current;
    const alive = () => run === generation.current;

    setFailure(null);
    setResults([]);
    setReportPath(null);
    setStatus("running");

    const collected = new Map<string, RunResult[]>();
    const scenes = new Map<string, Scene>();
    const record = (name: string, scenarioScene: Scene, stats: RunResult) => {
      const runs = collected.get(name) ?? [];
      runs.push(stats);
      collected.set(name, runs);
      scenes.set(name, scenarioScene);
    };

    const runOne = async (
      scenarioScene: Scene,
      scenarioMode: GlassMode,
      drag: boolean,
      durationMs: number,
    ) => {
      setScene(scenarioScene);
      setMode(scenarioMode);
      const video = videoRef.current;
      if (scenarioScene === "empty") {
        video?.pause();
      } else {
        await video?.play().catch(() => undefined);
      }
      await nextFrame();
      await nextFrame();

      const frameVideo = videoRef.current as FrameCallbackVideo | null;
      const supportsVideoFrames =
        typeof frameVideo?.requestVideoFrameCallback === "function";
      let presented = 0;
      let handle: number | null = null;
      const countFrame = () => {
        presented += 1;
        if (frameVideo?.requestVideoFrameCallback) {
          handle = frameVideo.requestVideoFrameCallback(countFrame);
        }
      };
      if (supportsVideoFrames && frameVideo && scenarioScene !== "empty") {
        handle = frameVideo.requestVideoFrameCallback(countFrame);
      }

      const startedAt = performance.now();
      const stats = await measure({
        durationMs,
        warmupMs: WARMUP_MS,
        onFrame: (elapsed) => {
          if (!drag) return;
          moveDragPanel(elapsed);
          if (scenarioScene === "stress") moveSecondPanel(elapsed);
        },
      });
      const elapsed = performance.now() - startedAt;
      if (handle !== null && frameVideo?.cancelVideoFrameCallback) {
        frameVideo.cancelVideoFrameCallback(handle);
      }

      return {
        ...stats,
        videoFrames: supportsVideoFrames ? presented : null,
        videoFps:
          supportsVideoFrames && elapsed > 0 ? (presented * 1000) / elapsed : null,
      } satisfies RunResult;
    };

    try {
      const baselineEmpty = await runOne("empty", "off", false, STATIC_MS);
      record("baseline-empty", "empty", baselineEmpty);

      const baselineVideo = await runOne("video", "off", false, STATIC_MS);
      record("baseline-video", "video", baselineVideo);

      for (let repeat = 0; repeat < REPEATS; repeat += 1) {
        const order: GlassMode[] = ["full", "lite", "off"];
        const offset = repeat % order.length;
        const rotated = [...order.slice(offset), ...order.slice(0, offset)];
        for (const scenarioMode of rotated) {
          if (!alive()) return;
          record(
            `drag-${scenarioMode}`,
            "panels",
            await runOne("panels", scenarioMode, true, MEASURE_MS),
          );
        }
      }

      for (let repeat = 0; repeat < REPEATS; repeat += 1) {
        if (!alive()) return;
        record(
          "drag-stress-full",
          "stress",
          await runOne("stress", "full", true, MEASURE_MS),
        );
      }

      for (const scenarioMode of ["full", "lite", "off"] as GlassMode[]) {
        if (!alive()) return;
        record(
          `static-${scenarioMode}`,
          "panels",
          await runOne("panels", scenarioMode, false, STATIC_MS),
        );
      }

      const scenarios: ScenarioResult[] = [...collected.entries()].map(([name, runs]) => ({
        name,
        scene: scenes.get(name) ?? "panels",
        mode: name.split("-")[1] as GlassMode,
        drag: name.startsWith("drag-"),
        runs,
        median: medianStats(runs),
        videoFps: median(
          runs
            .map((run) => run.videoFps)
            .filter((value): value is number => value !== null),
        ),
      }));

      if (!alive()) return;
      setResults(scenarios);
      setScene("panels");
      setMode("full");
      void videoRef.current?.play().catch(() => undefined);
      setStatus("saving");

      const report = {
        label: LABEL,
        createdAt: new Date().toISOString(),
        environment: environmentRef.current,
        userAgent: navigator.userAgent,
        viewport: {
          width: window.innerWidth,
          height: window.innerHeight,
          devicePixelRatio: window.devicePixelRatio,
        },
        visibility: document.visibilityState,
        glassEnabled: import.meta.env.VITE_GLASS_ENABLED !== "false",
        video: {
          ready: Boolean(videoRef.current && !videoRef.current.paused),
          source: videoRef.current?.currentSrc ?? null,
        },
        config: {
          repeats: REPEATS,
          measureMs: MEASURE_MS,
          warmupMs: WARMUP_MS,
          staticMs: STATIC_MS,
          blur: { full: { l1: 8, l2: 14, l3: 20 }, lite: 0, off: 0 },
          stressMovingPanels: 2,
        },
        scenarios,
      };

      const path = await saveReport(LABEL, report);
      if (!alive()) return;
      setReportPath(path);
      setStatus(path ? "done" : "done (report not saved)");
      window.__glassSpike = { report, path };
      console.info("[glass-spike] report", report);
    } catch (error) {
      if (alive()) {
        setFailure(String(error));
        setStatus("failed");
      }
    } finally {
      busy.current = false;
    }
  }, [moveDragPanel]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const environment = await probeEnvironment();
      if (cancelled) return;
      environmentRef.current = environment;
      setEnvironment(environment);
      tagGlassCapability(environment);

      setStatus("preparing window");
      await prepareWindow();
      if (cancelled) return;

      setStatus("warming video");
      const ready = await waitForVideo(videoRef.current);
      if (cancelled) return;
      if (!ready) {
        setFailure("The spike video did not start. Run scripts/make-glass-spike-video.sh.");
        setStatus("video missing");
        return;
      }
      if (AUTORUN) {
        await benchmark();
      } else {
        setStatus("idle");
      }
    })();

    return () => {
      cancelled = true;
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
    <main
      className="glass-spike"
      data-glass={mode}
      data-scene={scene}
      data-status={status}
    >
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

      <section className="glass spike-panel spike-l1" data-elev="1">
        <h2>Media</h2>
        <p>L1 / 8px blur</p>
      </section>
      <section className="glass spike-panel spike-l2" data-elev="2">
        <h2>Inspector</h2>
        <p>L2 / 14px blur</p>
      </section>
      <section className="glass spike-panel spike-l3" data-elev="3">
        <h2>Effects</h2>
        <p>L3 / 20px blur</p>
      </section>

      <div ref={dragRef} className="glass spike-panel spike-drag" data-elev="2">
        <h2>Draggable panel</h2>
        <p>This panel moves every animation frame.</p>
      </div>

      <div ref={drag2Ref} className="glass spike-panel spike-drag2" data-elev="3">
        <h2>Second draggable</h2>
        <p>Stress scene: two blurred layers recomposite per frame.</p>
      </div>

      <aside className="spike-hud">
        <div className="spike-hud-title">
          Glass spike · <span ref={liveRef}>--</span> fps live
        </div>
        <div>status: {status}</div>
        <div>
          scene: {scene} · mode: {mode}
        </div>
        {environment && (
          <div>
            {environment.gpuVendor} {environment.gpuClass} · {environment.gpuDriver} · WebKitGTK{" "}
            {environment.webkitgtkVersion}
          </div>
        )}
        {environment && (
          <div>
            DMA-BUF renderer {environment.dmabufRendererDisabled ? "disabled" : "enabled"} ·{" "}
            {environment.sessionType}
          </div>
        )}
        {summary.map((result) => (
          <div key={result.name} className="spike-result">
            {result.name}: <strong>{result.median.fps.toFixed(1)} fps</strong> · p95{" "}
            {result.median.p95.toFixed(1)}ms · long {result.median.longFrames.toFixed(0)}/
            {result.median.frames.toFixed(0)}
            {result.videoFps !== null && <> · video {result.videoFps.toFixed(1)} fps</>}
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
    __glassSpike?: { report: unknown; path: string | null };
  }
}
