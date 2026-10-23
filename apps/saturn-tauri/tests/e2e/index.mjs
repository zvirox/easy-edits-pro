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
import * as timeline from './timeline.mjs';
import * as audio from './audio.mjs';
import * as deliver from './deliver.mjs';
import * as media from './media.mjs';
import * as markers from './markers.mjs';
import * as motion from './motion.mjs';

// Area modules run in the original smoke order; each one receives the shared
// harness context and appends its results to `ctx.state`.
const areaModules = [shell, panels, timeline, audio, deliver, media, markers, motion];

const harness = await createHarness();
try {
  await harness.waitForServer();
  await installMockBackend(harness.page, {
    mediaUrl: harness.mockVideoUrl,
    catalogue: harness.catalogue,
  });
  for (const area of areaModules) {
    await area.run(harness);
  }

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
} finally {
  await harness.teardown();
}
