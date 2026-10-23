// Entry point for the Playwright UI smoke suite (`pnpm test:e2e`).
// IMPORTANT: this suite runs the UI against a MOCKED backend — the in-page
// Tauri mock in mock-backend.mjs answers every command from an in-memory
// document. It does not exercise the real Rust engine or GStreamer; native
// verification is covered separately (see docs and the F2 native workflow).
// See README.md next to this file for the module map.
import assert from 'node:assert/strict';
import { createHarness } from './harness.mjs';
import { installMockBackend } from './mock-backend.mjs';
import * as shell from './shell.mjs';
import * as panels from './panels.mjs';
import * as geometry from './geometry.mjs';
import * as timeline from './timeline.mjs';
import * as audio from './audio.mjs';
import * as deliver from './deliver.mjs';
import * as media from './media.mjs';
import * as markers from './markers.mjs';
import * as motion from './motion.mjs';

// Area modules run in the original smoke order; each one receives the shared
// harness context and appends its results to `ctx.state`.
const areaModules = [
  ['shell', shell],
  ['panels', panels],
  ['geometry', geometry],
  ['timeline', timeline],
  ['audio', audio],
  ['deliver', deliver],
  ['media', media],
  ['markers', markers],
  ['motion', motion],
];

const harness = await createHarness();

// A CI hang is worse than a CI failure: if any area stalls for 5 minutes,
// report the last reached area and exit non-zero instead of hanging forever.
const WATCHDOG_MS = 5 * 60_000;
let lastArea = 'setup';
const watchdog = setTimeout(() => {
  console.error(`[e2e] watchdog: stalled after area "${lastArea}"; failing the run.`);
  process.exit(1);
}, WATCHDOG_MS);

try {
  await harness.waitForServer();
  await installMockBackend(harness.page, {
    mediaUrl: harness.mockVideoUrl,
    catalogue: harness.catalogue,
  });
  for (const [name, area] of areaModules) {
    lastArea = name;
    console.error(`[e2e] running area: ${name}`);
    await area.run(harness);
  }
  lastArea = 'summary';

  assert.deepEqual(
    harness.pageErrors,
    [],
    `the app should not produce browser errors: ${harness.pageErrors.join('\n')}`,
  );
  const state = harness.state;
  await harness.page.screenshot({ path: '/tmp/easy-edits-pro-editor-smoke.png', fullPage: false });
  console.log(JSON.stringify({
    page: await harness.page.url(),
    viewport: harness.page.viewportSize(),
    welcome: 'passed',
    importWorkspace: 'passed',
    importMediaSelection: 'passed',
    audioWorkspace: 'passed',
    audioDspCommand: state.audioCommand,
    exportWorkspace: 'passed',
    hardwareEncoderFormatToggle: 'passed',
    mediaPointerDropPx: state.initialLeft,
    zoomWidth: state.zoomWidth,
    scrubSeekSeconds: state.seekTime,
    playbackAdvancedToSeconds: state.playingTime,
    nextFrameSeconds: state.nextFrameTime,
    previousFrameSeconds: state.previousFrameTime,
    movedClipLeftPx: state.movedLeft,
    markerRequests: state.markerRequests,
    markerCount: state.markerCount,
    mediaOrganisation: state.mediaOrganisation,
    reducedMotion: state.reducedMotion,
    editorScreenshot: '/tmp/easy-edits-pro-editor-smoke.png',
    importScreenshot: '/tmp/easy-edits-pro-import-workspace.png',
    exportScreenshot: '/tmp/easy-edits-pro-export-workspace.png',
    browserErrors: harness.pageErrors.length,
  }, null, 2));
  clearTimeout(watchdog);
  // Never let lingering stdio pipes to the (now killed) dev server keep this
  // process alive after a fully green run.
  process.exit(0);
} catch (error) {
  clearTimeout(watchdog);
  throw error;
} finally {
  await harness.teardown();
}
