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

try {
  await waitForServer();
  await page.addInitScript(({ mediaUrl }) => {
    const second = 254016000000;
    let doc = null;
    let currentPath = null;
    let nextId = 1;
    const undoStack = [];
    const snapshot = () => ({ document: structuredClone(doc), path: currentPath, is_dirty: undoStack.length > 0 });
    const push = () => undoStack.push(structuredClone(doc));
    const execute = request => {
      const { id, params = {} } = request;
      if (id === 'project.status') return snapshot();
      if (id === 'project.new') {
        doc = { format: 'project-saturn.project', schema_version: 4, generator: 'browser-smoke-test', project: {
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
        track.clips.push({ media_id: media.id, timeline_start: start, source_in: 0, duration: media.duration ?? 6 * second, color: { exposure: 0, contrast: 1, saturation: 1 }, keyframes: [] });
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
      if (id === 'edit.undo') {
        if (undoStack.length) doc = undoStack.pop();
        return { undone: true };
      }
      return { ok: true };
    };
    const catalogue = [
      ['project.new','New Project',['File'],'Ctrl+N'], ['file.open','Open Project',['File'],'Ctrl+O'], ['file.save','Save Project',['File'],'Ctrl+S'],
      ['project.add_media','Import Media',['File','Import Media'],'Ctrl+I'], ['edit.undo','Undo',['Edit'],'Ctrl+Z'], ['edit.redo','Redo',['Edit'],'Ctrl+Shift+Z'],
      ['timeline.add_clip','Add Clip to Timeline',['Sequence'],null], ['timeline.remove_clip','Remove Clip',['Clip'],null], ['timeline.move_clip','Move Clip',['Timeline'],null],
      ['timeline.split_clip','Split Clip at Playhead',['Clip'],null], ['timeline.ripple_delete','Ripple Delete',['Clip'],null],
    ].map(([id,label,menu,shortcut]) => ({ id,label,menu,shortcut,mutating:true }));
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
        if (command === 'get_media_server_port') return 0;
        if (command === 'get_all_jobs') return [];
        if (command === 'plugin:dialog|save') return '/tmp/browser-test.saturn';
        if (command === 'plugin:dialog|open') return ['/mock/editor-smoke.webm'];
        if (command.startsWith('plugin:window|')) return null;
        throw new Error(`Unhandled mocked Tauri command: ${command}`);
      },
    };
  }, { mediaUrl: mockVideoUrl });

  await page.goto('http://127.0.0.1:1420/');
  assert.equal(await page.title(), 'Easy Edits Pro', 'the intended app page should load');
  assert.match(await page.locator('body').innerText(), /Make room/, 'the welcome screen should render');
  await page.getByRole('button', { name: /Create a new project/ }).click();
  await page.getByRole('button', { name: /Create project/ }).click();
  await page.locator('.app-shell').waitFor();
  await page.getByRole('button', { name: /Import media/ }).first().click();
  await page.locator('.media-item strong', { hasText: 'editor-smoke.webm' }).waitFor();

  await page.getByRole('navigation', { name: 'Main workspaces' }).getByRole('button', { name: 'Import' }).click();
  await page.getByRole('heading', { name: 'Import' }).waitFor();
  await page.getByText('Bring footage, audio, and graphics into this project.').waitFor();
  await page.locator('.import-media-card').waitFor();
  await page.screenshot({ path: '/tmp/easy-edits-pro-import-workspace.png', fullPage: false });
  await page.getByRole('button', { name: /editor-smoke\.webm/ }).click();
  await page.getByRole('button', { name: 'Edit', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Edit', exact: true }).click();

  // Use actual pointer events to place Project media onto the V1 timeline lane.
  const mediaItem = page.locator('.media-item').first();
  const mediaBounds = await mediaItem.boundingBox();
  const videoLane = page.locator('.track-lane[data-track-id="1"]');
  const laneBounds = await videoLane.boundingBox();
  await page.mouse.move(mediaBounds.x + mediaBounds.width / 2, mediaBounds.y + mediaBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(laneBounds.x + 60, laneBounds.y + laneBounds.height / 2, { steps: 12 });
  await page.mouse.up();
  await page.getByText('Clip added at drop position').waitFor();
  const clip = page.locator('.timeline-clip').first();
  const initialLeft = Number.parseFloat(await clip.evaluate(element => element.style.left));
  assert.ok(initialLeft > 30, `pointer drop should place the clip at the pointer position; got ${initialLeft}px`);

  const timeline = page.locator('.timeline-scroller');
  const widthBefore = await timeline.evaluate(element => element.scrollWidth);
  await page.getByRole('button', { name: 'Zoom in' }).click();
  const widthAfter = await timeline.evaluate(element => element.scrollWidth);
  assert.ok(widthAfter > widthBefore, 'zoom in should expand timeline content');

  const playhead = page.getByRole('slider', { name: 'Timeline playhead' }).first();
  const gripBounds = await playhead.boundingBox();
  const rulerBounds = await page.locator('.ruler-lane').boundingBox();
  await page.mouse.move(gripBounds.x + gripBounds.width / 2, gripBounds.y + gripBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(rulerBounds.x + 250, gripBounds.y + gripBounds.height / 2, { steps: 8 });
  await page.mouse.up();
  const programMonitor = page.locator('.monitor').filter({ hasText: 'Program' });
  const video = programMonitor.locator('video');
  await video.waitFor();
  await page.waitForFunction(() => {
    const program = Array.from(document.querySelectorAll('.monitor')).find(panel => panel.textContent?.includes('Program'));
    const currentTime = Number(program?.querySelector('video')?.currentTime ?? 0);
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
  const pausedTime = await video.evaluate(element => element.currentTime);
  await page.getByRole('button', { name: 'Next frame' }).click();
  await page.waitForFunction(minimum => {
    const program = Array.from(document.querySelectorAll('.monitor')).find(panel => panel.textContent?.includes('Program'));
    return Number(program?.querySelector('video')?.currentTime ?? 0) > minimum;
  }, pausedTime);
  const nextFrameTime = await video.evaluate(element => element.currentTime);
  assert.ok(nextFrameTime > pausedTime, `next-frame should advance the preview; ${pausedTime}s -> ${nextFrameTime}s`);
  await page.getByRole('button', { name: 'Previous frame' }).click();
  await page.waitForFunction(maximum => {
    const program = Array.from(document.querySelectorAll('.monitor')).find(panel => panel.textContent?.includes('Program'));
    return Number(program?.querySelector('video')?.currentTime ?? 0) < maximum;
  }, nextFrameTime);
  const previousFrameTime = await video.evaluate(element => element.currentTime);
  assert.ok(previousFrameTime < nextFrameTime, `previous-frame should move the preview backward; ${nextFrameTime}s -> ${previousFrameTime}s`);

  const workspaceNav = page.getByRole('navigation', { name: 'Main workspaces' });
  await workspaceNav.getByRole('button', { name: 'Export' }).click();
  await page.getByRole('heading', { name: 'Media File' }).waitFor();
  await page.getByText('Set a format and size that match where you’ll share this edit.').waitFor();
  const hardwareEncoding = page.getByRole('checkbox', { name: /Use hardware encoder when available/ });
  assert.ok(await hardwareEncoding.isEnabled(), 'hardware encoding should be selectable for MP4');
  await page.getByLabel('Format').selectOption('webm');
  assert.equal(await hardwareEncoding.isEnabled(), false, 'hardware H.264 encoding should be disabled for WebM');
  await page.getByLabel('Format').selectOption('mp4');
  await page.waitForFunction(() => {
    const video = document.querySelector('.export-preview-stage video');
    return video && video.readyState >= 2 && video.videoWidth > 0;
  });
  await delay(100);
  await page.screenshot({ path: '/tmp/easy-edits-pro-export-workspace.png', fullPage: false });
  await workspaceNav.getByRole('button', { name: 'Edit', exact: true }).click();

  const clipBounds = await clip.boundingBox();
  await page.mouse.move(clipBounds.x + 50, clipBounds.y + 18);
  await page.mouse.down();
  await page.mouse.move(clipBounds.x + 170, clipBounds.y + 18, { steps: 8 });
  await page.mouse.up();
  const movedLeft = Number.parseFloat(await page.locator('.timeline-clip').first().evaluate(element => element.style.left));
  assert.ok(movedLeft > initialLeft + 80, `clip pointer move should update its timeline start; ${initialLeft}px -> ${movedLeft}px`);

  assert.deepEqual(pageErrors, [], `the app should not produce browser errors: ${pageErrors.join('\n')}`);
  await page.screenshot({ path: '/tmp/easy-edits-pro-editor-smoke.png', fullPage: false });
  console.log(JSON.stringify({ page: await page.url(), viewport: page.viewportSize(), welcome: 'passed', importWorkspace: 'passed', importMediaSelection: 'passed', exportWorkspace: 'passed', hardwareEncoderFormatToggle: 'passed', mediaPointerDropPx: initialLeft, zoomWidth: { before: widthBefore, after: widthAfter }, scrubSeekSeconds: seekTime, playbackAdvancedToSeconds: playingTime, nextFrameSeconds: nextFrameTime, previousFrameSeconds: previousFrameTime, movedClipLeftPx: movedLeft, editorScreenshot: '/tmp/easy-edits-pro-editor-smoke.png', importScreenshot: '/tmp/easy-edits-pro-import-workspace.png', exportScreenshot: '/tmp/easy-edits-pro-export-workspace.png', browserErrors: pageErrors.length }, null, 2));
} finally {
  await browser.close();
  server.kill('SIGTERM');
}
