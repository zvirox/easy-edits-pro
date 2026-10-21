import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { chromium } from 'playwright';

const appDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const fixture = await readFile(path.join(appDirectory, 'tests/fixtures/editor-smoke.webm'));
const mockVideoUrl = `data:video/webm;base64,${fixture.toString('base64')}`;
const catalogue = JSON.parse(await readFile(path.join(appDirectory, '../../docs/reference/command-catalogue.json'), 'utf8'));
const server = spawn('pnpm', ['dev', '--host', '127.0.0.1', '--strictPort'], {
  cwd: appDirectory,
  stdio: ['ignore', 'pipe', 'pipe'],
});
let serverOutput = '';
server.stdout.on('data', chunk => { serverOutput += chunk.toString(); });
server.stderr.on('data', chunk => { serverOutput += chunk.toString(); });

async function waitForServer() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) throw new Error(`Vite exited early:\n${serverOutput}`);
    try {
      const response = await fetch('http://127.0.0.1:1420/');
      if (response.ok) return;
    } catch { /* Vite is not listening yet. */ }
    await delay(200);
  }
  throw new Error(`Timed out waiting for Vite at 127.0.0.1:1420:\n${serverOutput}`);
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
page.setDefaultTimeout(8_000);
const pageErrors = [];
page.on('pageerror', error => pageErrors.push(error.message));
page.on('console', message => { if (message.type() === 'error') pageErrors.push(message.text()); });
await page.route('http://127.0.0.1:1421/media**', async route => {
  const range = route.request().headers().range;
  const match = range?.match(/^bytes=(\d+)-(\d*)$/);
  const start = match ? Number(match[1]) : 0;
  const end = match?.[2] ? Math.min(Number(match[2]), fixture.length - 1) : fixture.length - 1;
  const headers = {
    'Accept-Ranges': 'bytes',
    'Access-Control-Allow-Origin': 'http://127.0.0.1:1420',
    'Content-Type': 'video/webm',
  };
  if (match) {
    headers['Content-Range'] = `bytes ${start}-${end}/${fixture.length}`;
    await route.fulfill({ status: 206, headers, body: fixture.subarray(start, end + 1) });
  } else {
    await route.fulfill({ headers, body: fixture });
  }
});

try {
  await waitForServer();
  await page.addInitScript(({ mediaUrl, catalogue }) => {
    const second = 254016000000;
    let doc = null;
    let currentPath = null;
    let nextId = 1;
    const undoStack = [];
    window.__commandRequests = [];
    const snapshot = () => ({ document: structuredClone(doc), path: currentPath, is_dirty: undoStack.length > 0 });
    const push = () => undoStack.push(structuredClone(doc));
    const execute = request => {
      const { id, params = {} } = request;
      window.__commandRequests.push({ id, params });
      if (id === 'project.status') return snapshot();
      if (id === 'project.new') {
        doc = { format: 'project-saturn.project', schema_version: 5, generator: 'browser-smoke-test', project: {
          name: params.name ?? 'Browser Test', settings: { width: params.width ?? 1920, height: params.height ?? 1080, frame_rate: params.frame_rate ?? { numerator: 30, denominator: 1 }, audio_sample_rate: 48000 },
          media: [], next_id: 3, sequence: { name: params.sequence_name ?? 'Sequence 1', mark_in: null, mark_out: null, tracks: [
            { id: 1, kind: 'video', name: 'V1', gain_db: 0, clips: [] }, { id: 2, kind: 'audio', name: 'A1', gain_db: 0, clips: [] },
          ] },
        }, view: {} };
        currentPath = null;
        undoStack.length = 0;
        return { created: true };
      }
      if (id === 'file.save') { currentPath = params.path; return { saved: true }; }
      if (id === 'timeline.add_clip') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const media = doc.project.media.find(item => item.id === params.media_id);
        const start = params.timeline_start ?? track.clips.reduce((end, clip) => Math.max(end, clip.timeline_start + clip.duration), 0);
        track.clips.push({ media_id: media.id, timeline_start: start, source_in: 0, duration: media.duration ?? 6 * second, color: { exposure: 0, contrast: 1, saturation: 1 }, effects: [], keyframes: [] });
        track.clips.sort((a, b) => a.timeline_start - b.timeline_start);
        return { track_id: track.id, clip_index: track.clips.findIndex(clip => clip.media_id === media.id && clip.timeline_start === start) };
      }
      if (id === 'timeline.move_clip') {
        push();
        const source = doc.project.sequence.tracks.find(track => track.id === params.track_id);
        const [clip] = source.clips.splice(params.clip_index, 1);
        const target = doc.project.sequence.tracks.find(track => track.id === params.target_track_id);
        clip.timeline_start = params.timeline_start;
        target.clips.push(clip);
        target.clips.sort((a, b) => a.timeline_start - b.timeline_start);
        return { track_id: target.id, clip_index: target.clips.indexOf(clip), moved: true };
      }
      if (id === 'timeline.split_clip') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const clip = track.clips[params.clip_index];
        const delta = params.at - clip.timeline_start;
        const right = structuredClone(clip);
        right.timeline_start = params.at;
        right.source_in += delta;
        right.duration -= delta;
        clip.duration = delta;
        track.clips.splice(params.clip_index + 1, 0, right);
        return { split: true };
      }
      if (id === 'timeline.remove_clip' || id === 'timeline.ripple_delete') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        track.clips.splice(params.clip_index, 1);
        return { removed: true };
      }
      if (id === 'track.set_audio_processing') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const { track_id: _trackId, ...processing } = params;
        track.audio_processing = { ...track.audio_processing, ...processing };
        return { updated: true };
      }
      if (id === 'edit.undo') {
        if (undoStack.length) doc = undoStack.pop();
        return { undone: true };
      }
      return { ok: true };
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' } },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      convertFileSrc: () => mediaUrl,
      invoke: async (command, args = {}) => {
        if (command === 'recovery_status') return { available: false };
        if (command === 'get_command_catalogue') return catalogue;
        if (command === 'command_status') return { enabled: args.id === 'edit.undo', reason: null };
        if (command === 'check_media_availability') return (doc?.project.media ?? []).map(media => ({ mediaId: media.id, missing: false }));
        if (command === 'prepare_video_preview') return '/mock/editor-smoke-preview.webm';
        if (command === 'execute_command') return execute(args.request);
        if (command === 'import_media') {
          for (const mediaPath of args.paths ?? []) doc.project.media.push({ id: nextId++, name: mediaPath.split('/').at(-1), path: mediaPath, kind: 'video', duration: 6 * second, width: 640, height: 360 });
          return snapshot();
        }
        if (command === 'get_media_server_port') return 1421;
        if (command === 'get_media_server_token') return 'browser-smoke-token';
        if (command === 'get_all_jobs') return [];
        if (command === 'plugin:event|listen') return 1;
        if (command === 'plugin:event|unlisten') return null;
        if (command === 'plugin:dialog|save') return '/tmp/browser-test.saturn';
        if (command === 'plugin:dialog|open') return ['/mock/editor-smoke.webm'];
        if (command.startsWith('plugin:window|')) return null;
        throw new Error(`Unhandled mocked Tauri command: ${command}`);
      },
    };
  }, { mediaUrl: mockVideoUrl, catalogue });

  await page.goto('http://127.0.0.1:1420/');
  assert.equal(await page.title(), 'Easy Edits Pro', 'the intended app page should load');
  assert.match(await page.locator('body').innerText(), /Make room/, 'the welcome screen should render');
  await page.getByRole('button', { name: /Create a new project/ }).click();
  await page.getByRole('button', { name: /Create project/ }).click();
  await page.locator('.app-shell').waitFor();

  // Menus are generated from the Rust command catalogue: every top-level menu
  // must list commands, and Window must expose the workspace switcher.
  const menuBar = page.locator('.menu-bar');
  for (const menu of ['File', 'Edit', 'Clip', 'Markers', 'View', 'Window', 'Help']) {
    await menuBar.getByRole('button', { name: menu, exact: true }).click();
    const popover = page.locator('.menu-popover').last();
    await popover.waitFor();
    const entries = await popover.locator('button').count();
    assert.ok(entries > 0, `the ${menu} menu should list at least one command`);
    await page.keyboard.press('Escape');
    await popover.waitFor({ state: 'detached' });
  }
  for (const emptyMenu of ['Sequence', 'Graphics', 'Audio', 'Effects']) {
    assert.equal(
      await menuBar.getByRole('button', { name: emptyMenu, exact: true }).count(),
      0,
      `the ${emptyMenu} menu has no shipped commands yet and must not render`,
    );
  }
  // The command palette reads the same catalogue.
  await page.keyboard.press('Control+k');
  await page.locator('#command-palette-search').fill('ripple');
  await page.locator('.command-palette-result-label', { hasText: 'Ripple Delete' }).first().waitFor();
  await page.keyboard.press('Escape');

  // UI-only commands dispatch locally instead of hitting the engine.
  await menuBar.getByRole('button', { name: 'View', exact: true }).click();
  await page.locator('.menu-popover').getByText('Theme: Light', { exact: true }).click();
  assert.equal(
    await page.evaluate(() => document.documentElement.dataset.theme),
    'light',
    'View ▸ Theme: Light should switch the theme',
  );
  await menuBar.getByRole('button', { name: 'View', exact: true }).click();
  await page.locator('.menu-popover').getByText('Theme: Dark', { exact: true }).click();

  // Window ▸ Workspaces switches the routed workspace.
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Color', { exact: true }).click();
  await page.locator('.color-workspace-page').waitFor();
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Edit', { exact: false }).click();
  await page.locator('.app-shell').waitFor();
  // Shortcut-dispatched commands receive playhead context.
  await page.locator('.app-shell').press('i');
  const markRequests = await page.evaluate(() => window.__commandRequests);
  assert.ok(
    markRequests.some(
      (request) => request.id === 'timeline.set_mark_in' && typeof request.params?.at === 'number',
    ),
    'I should set the in point with the playhead tick position',
  );

  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Edit').waitFor();
  // Let the menu fade-in finish so the artifact matches the settled UI.
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-menu.png', fullPage: false });
  await page.keyboard.press('Escape');

  await page.locator('button[data-tooltip="Import Media Files (Ctrl+I)"]').first().click();
  await page.locator('.media-item strong', { hasText: 'editor-smoke.webm' }).waitFor();

  const workspaces = page.getByRole('radiogroup', { name: 'Workspaces' });
  await workspaces.getByRole('radio', { name: 'Media' }).click();
  await page.getByText('MEDIA POOL').waitFor();
  await page.getByText('Add media to your project').waitFor();
  await page.locator('.import-media-card').waitFor();
  await page.screenshot({ path: '/tmp/easy-edits-pro-import-workspace.png', fullPage: false });
  await page.getByRole('button', { name: /editor-smoke\.webm/ }).click();
  await workspaces.getByRole('radio', { name: 'Edit' }).click();

  // Use actual pointer events to place Project media onto the V1 timeline lane.
  const mediaItem = page.locator('.media-item').first();
  const mediaBounds = await mediaItem.boundingBox();
  const videoLane = page.locator('.track-lane[data-track-id="1"]');
  const laneBounds = await videoLane.boundingBox();
  await page.mouse.move(mediaBounds.x + mediaBounds.width / 2, mediaBounds.y + mediaBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(laneBounds.x + 60, laneBounds.y + laneBounds.height / 2, { steps: 12 });
  await page.mouse.up();
  await page.getByText('Added editor-smoke.webm to timeline').waitFor();
  const clip = page.locator('.timeline-clip').first();
  const initialLeft = Number.parseFloat(await clip.evaluate(element => element.style.left));
  assert.ok(initialLeft > 30, `pointer drop should place the clip at the pointer position; got ${initialLeft}px`);

  const timeline = page.locator('.timeline-scroller');
  const widthBefore = await timeline.evaluate(element => element.scrollWidth);
  await page.getByRole('button', { name: 'Zoom in' }).click();
  const widthAfter = await timeline.evaluate(element => element.scrollWidth);
  assert.ok(widthAfter > widthBefore, 'zoom in should expand timeline content');

  await page.locator('.ruler-lane').click({ position: { x: 250, y: 8 } });
  const programMonitor = page.locator('.monitor').filter({ hasText: 'Program' });
  const video = programMonitor.locator('video.program-deck:not(.deck-hidden)');
  await video.waitFor();
  await page.waitForFunction(() => {
    const program = Array.from(document.querySelectorAll('.monitor')).find(panel => panel.textContent?.includes('Program'));
    const currentTime = Number(program?.querySelector('video.program-deck:not(.deck-hidden)')?.currentTime ?? 0);
    return currentTime > 1.5;
  });
  const seekTime = await video.evaluate(element => element.currentTime);
  assert.ok(seekTime > 1.5, `playhead scrub should seek Program media; got ${seekTime}s`);

  await page.getByRole('button', { name: 'Play clip' }).click();
  await page.getByRole('button', { name: 'Pause playback' }).waitFor();
  await delay(500);
  const playingTime = await video.evaluate(element => element.currentTime);
  assert.ok(playingTime > seekTime, `playback should advance the media clock; ${seekTime}s -> ${playingTime}s`);
  await page.getByRole('button', { name: 'Pause playback' }).click();
  await page.getByRole('button', { name: 'Play clip' }).waitFor();
  const timelinePlayhead = page.locator('.ruler-lane .playhead-grip');
  const pausedTime = Number(await timelinePlayhead.getAttribute('aria-valuenow'));
  await page.getByRole('button', { name: 'Next frame' }).click();
  await page.waitForFunction(minimum => {
    return Number(document.querySelector('.ruler-lane .playhead-grip')?.getAttribute('aria-valuenow') ?? 0) > minimum;
  }, pausedTime);
  const nextFrameTime = Number(await timelinePlayhead.getAttribute('aria-valuenow'));
  assert.ok(nextFrameTime > pausedTime, `next-frame should advance the preview; ${pausedTime}s -> ${nextFrameTime}s`);
  await page.getByRole('button', { name: 'Previous frame' }).click();
  await page.waitForFunction(maximum => {
    return Number(document.querySelector('.ruler-lane .playhead-grip')?.getAttribute('aria-valuenow') ?? 0) < maximum;
  }, nextFrameTime);
  const previousFrameTime = Number(await timelinePlayhead.getAttribute('aria-valuenow'));
  assert.ok(previousFrameTime < nextFrameTime, `previous-frame should move the preview backward; ${nextFrameTime}s -> ${previousFrameTime}s`);

  const audioWorkspaceNav = page.getByRole('radiogroup', { name: 'Workspaces' });
  await audioWorkspaceNav.getByRole('radio', { name: 'Fairlight' }).click();
  await page.getByText('FAIRLIGHT SOUND SUITE').waitFor();
  await page.getByText('CHANNEL STRIP RACK').waitFor();
  await page.locator('.eq-module .dsp-power-btn').click();
  await page.waitForFunction(() => Array.from(document.querySelectorAll('.eq-module .dsp-power-btn')).some(button => button.textContent?.trim() === 'ACTIVE'));
  const audioCommand = await page.evaluate(() => window.__commandRequests.findLast(item => item.id === 'track.set_audio_processing'));
  assert.equal(audioCommand?.params?.track_id, 2, 'Fairlight EQ toggle should update the selected audio track');
  assert.equal(audioCommand?.params?.eq_enabled, true, 'Fairlight EQ toggle should send the enabled state to the editor engine');
  assert.ok(await page.locator('.fairlight-channel-strip').count() >= 2, 'Fairlight should display the project video and audio channel strips');

  const workspaceNav = page.getByRole('radiogroup', { name: 'Workspaces' });
  await workspaceNav.getByRole('radio', { name: 'Deliver' }).click();
  await page.getByText('Media File', { exact: true }).waitFor();
  const hardwareEncoding = page.getByRole('checkbox', { name: /Use hardware encoder when available/ });
  assert.ok(await hardwareEncoding.isEnabled(), 'hardware encoding should be selectable for MP4');
  await page.getByLabel('Format').selectOption('webm');
  assert.equal(await hardwareEncoding.isEnabled(), false, 'hardware H.264 encoding should be disabled for WebM');
  await page.getByLabel('Format').selectOption('mp4');
  await delay(100);
  await page.screenshot({ path: '/tmp/easy-edits-pro-export-workspace.png', fullPage: false });
  await workspaceNav.getByRole('radio', { name: 'Edit' }).click();

  const clipBounds = await clip.boundingBox();
  await page.mouse.move(clipBounds.x + 50, clipBounds.y + 18);
  await page.mouse.down();
  await page.mouse.move(clipBounds.x + 170, clipBounds.y + 18, { steps: 8 });
  await page.mouse.up();
  const movedLeft = Number.parseFloat(await page.locator('.timeline-clip').first().evaluate(element => element.style.left));
  assert.ok(movedLeft > initialLeft + 80, `clip pointer move should update its timeline start; ${initialLeft}px -> ${movedLeft}px`);

  // Phase 3: reduced-motion is a hard contract. Emulating `prefers-reduced-motion:
  // reduce` must match the media query and collapse every CSS animation to an
  // instant frame, while mount/unmount surfaces (command palette) still work.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const reducedMotion = await page.evaluate(() => {
    const probe = document.createElement('div');
    probe.className = 'preview-preparing-dot';
    document.body.appendChild(probe);
    const raw = getComputedStyle(probe).animationDuration;
    probe.remove();
    const seconds = raw.endsWith('ms') ? Number.parseFloat(raw) / 1000 : Number.parseFloat(raw);
    return {
      mediaQueryMatched: window.matchMedia('(prefers-reduced-motion: reduce)').matches,
      probeAnimationSeconds: seconds,
      raw,
    };
  });
  assert.equal(reducedMotion.mediaQueryMatched, true, 'emulated reduced motion should match the media query');
  assert.ok(reducedMotion.probeAnimationSeconds <= 0.05, `reduced motion should collapse CSS animations; ${reducedMotion.raw}`);
  await page.keyboard.press('ControlOrMeta+k');
  await page.locator('.command-palette').waitFor();
  await page.keyboard.press('Escape');
  await page.locator('.command-palette').waitFor({ state: 'detached' });
  await page.emulateMedia({ reducedMotion: 'no-preference' });

  assert.deepEqual(pageErrors, [], `the app should not produce browser errors: ${pageErrors.join('\n')}`);
  await page.screenshot({ path: '/tmp/easy-edits-pro-editor-smoke.png', fullPage: false });
  console.log(JSON.stringify({ page: await page.url(), viewport: page.viewportSize(), welcome: 'passed', importWorkspace: 'passed', importMediaSelection: 'passed', audioWorkspace: 'passed', audioDspCommand: audioCommand, exportWorkspace: 'passed', hardwareEncoderFormatToggle: 'passed', mediaPointerDropPx: initialLeft, zoomWidth: { before: widthBefore, after: widthAfter }, scrubSeekSeconds: seekTime, playbackAdvancedToSeconds: playingTime, nextFrameSeconds: nextFrameTime, previousFrameSeconds: previousFrameTime, movedClipLeftPx: movedLeft, reducedMotion, editorScreenshot: '/tmp/easy-edits-pro-editor-smoke.png', importScreenshot: '/tmp/easy-edits-pro-import-workspace.png', exportScreenshot: '/tmp/easy-edits-pro-export-workspace.png', browserErrors: pageErrors.length }, null, 2));
} finally {
  await browser.close();
  server.kill('SIGTERM');
}
