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
          media: [], next_id: 3, sequences: [], sequence: { id: 1, name: params.sequence_name ?? 'Sequence 1', mark_in: null, mark_out: null, tracks: [
            { id: 1, kind: 'video', name: 'V1', gain_db: 0, clips: [] }, { id: 2, kind: 'audio', name: 'A1', gain_db: 0, clips: [] },
          ] },
        }, view: {} };
        currentPath = null;
        undoStack.length = 0;
        return { created: true };
      }
      if (id === 'file.save') { currentPath = params.path; return { saved: true }; }
      if (id === 'media.create_bin') {
        push();
        doc.project.bins ??= [];
        const binId = (doc.project.next_id = (doc.project.next_id ?? 1) + 1);
        const bin = { id: binId, name: params.name, parent_id: params.parent_id ?? null, color: params.color ?? '#38bdf8' };
        doc.project.bins.push(bin);
        return { created: true, bin };
      }
      if (id === 'media.rename_bin') {
        push();
        const bin = (doc.project.bins ?? []).find(item => item.id === params.bin_id);
        if (!bin) throw new Error(`Bin ${params.bin_id} not found`);
        bin.name = params.name;
        return { renamed: true, bin_id: bin.id, name: bin.name };
      }
      if (id === 'media.delete_bin') {
        push();
        doc.project.bins = (doc.project.bins ?? []).filter(item => item.id !== params.bin_id);
        for (const item of doc.project.media ?? []) {
          if (item.bin_id === params.bin_id) item.bin_id = null;
        }
        return { deleted: true, bin_id: params.bin_id };
      }
      if (id === 'media.move_to_bin') {
        push();
        const item = doc.project.media.find(entry => entry.id === params.media_id);
        if (!item) throw new Error(`Media ${params.media_id} not found`);
        item.bin_id = params.bin_id ?? null;
        return { moved: true, media_id: item.id, bin_id: item.bin_id };
      }
      if (id === 'media.update_properties') {
        push();
        const item = doc.project.media.find(entry => entry.id === params.media_id);
        if (!item) throw new Error(`Media ${params.media_id} not found`);
        if (params.name) item.name = params.name;
        if (params.color_label !== undefined) item.color_label = params.color_label;
        if (params.tags !== undefined) item.tags = params.tags;
        for (const field of ['comment', 'scene', 'take']) {
          if (params[field] !== undefined) item[field] = params[field];
        }
        return { updated: true, media_id: item.id, name: item.name, scene: item.scene ?? '', take: item.take ?? '', comment: item.comment ?? '' };
      }
      if (id === 'marker.add' || id === 'marker.add_range') {
        push();
        doc.project.sequence.markers ??= [];
        const markerId = (doc.project.next_id = (doc.project.next_id ?? 1) + 1);
        const start = id === 'marker.add' ? (params.at ?? 0) : params.start;
        const duration = id === 'marker.add' ? (params.duration ?? 0) : params.end - params.start;
        doc.project.sequence.markers.push({
          id: markerId,
          name: params.name ?? `Marker ${doc.project.sequence.markers.length + 1}`,
          kind: params.kind ?? 'comment',
          start_ticks: start,
          duration_ticks: duration,
          color: params.color ?? '',
          note: params.note ?? '',
        });
        doc.project.sequence.markers.sort((a, b) => a.start_ticks - b.start_ticks);
        return { added: true, id: markerId, range: duration > 0 };
      }
      if (id === 'marker.next' || id === 'marker.previous') {
        const ordered = [...(doc.project.sequence.markers ?? [])].sort((a, b) => a.start_ticks - b.start_ticks);
        const marker = id === 'marker.next'
          ? ordered.find(item => item.start_ticks > params.from)
          : [...ordered].reverse().find(item => item.start_ticks < params.from);
        return marker ? { target: marker.start_ticks, id: marker.id, name: marker.name } : { target: null };
      }
      if (id === 'marker.update') {
        push();
        const marker = (doc.project.sequence.markers ?? []).find(item => item.id === params.id);
        if (!marker) throw new Error(`Marker ${params.id} does not exist`);
        for (const key of ['name', 'kind', 'color', 'note']) {
          if (params[key] !== undefined) marker[key] = params[key];
        }
        return { updated: true, id: marker.id };
      }
      if (id === 'marker.remove') {
        push();
        const markers = doc.project.sequence.markers ?? [];
        const index = markers.findIndex(item => item.id === params.id);
        if (index >= 0) markers.splice(index, 1);
        return { removed: index >= 0, id: params.id };
      }
      if (id === 'marker.clear') {
        push();
        const cleared = (doc.project.sequence.markers ?? []).length;
        doc.project.sequence.markers = [];
        return { cleared };
      }
      if (id === 'marker.split') {
        push();
        let split = 0;
        const markers = doc.project.sequence.markers ?? [];
        for (const marker of [...markers]) {
          if (marker.duration_ticks <= 0) continue;
          if (params.at <= marker.start_ticks || params.at >= marker.start_ticks + marker.duration_ticks) continue;
          const tail = { ...marker, id: (doc.project.next_id = (doc.project.next_id ?? 1) + 1), start_ticks: params.at, duration_ticks: marker.start_ticks + marker.duration_ticks - params.at };
          marker.duration_ticks = params.at - marker.start_ticks;
          markers.push(tail);
          split += 1;
        }
        return { split };
      }
      if (id === 'marker.export_csv') {
        const markers = doc.project.sequence.markers ?? [];
        return { csv: 'name,kind,start_ticks,duration_ticks,color,note\n', count: markers.length };
      }
      if (id === 'marker.import_csv') {
        push();
        return { imported: 0, replaced: Boolean(params.replace) };
      }
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
      if (id === 'timeline.rate_stretch') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const clip = track.clips[params.clip_index];
        const oldDuration = clip.duration;
        const oldSpeed = clip.speed > 0 ? clip.speed : 1;
        let newSpeed = params.speed ?? oldSpeed;
        let newDuration = params.new_duration ??
          Math.round((oldDuration * oldSpeed) / newSpeed);
        if (!params.new_duration) {
          newSpeed = params.speed;
        } else {
          newSpeed = (oldDuration * oldSpeed) / newDuration;
        }
        clip.duration = Math.max(1, newDuration);
        clip.speed = Math.round(newSpeed * 1000) / 1000;
        return {
          stretched: true,
          duration: clip.duration,
          speed: clip.speed,
          linked: Boolean(params.linked),
          affected: 1,
        };
      }
      if (id === 'sequence.create' || id === 'sequence.duplicate' || id === 'sequence.rename' || id === 'sequence.delete' || id === 'project.switch_sequence') {
        const allocId = () => {
          const value = doc.project.next_id ?? 1;
          doc.project.next_id = value + 1;
          return value;
        };
        const active = doc.project.sequence;
        doc.project.sequences ??= [];
        push();
        if (id === 'sequence.create') {
          const sequence = {
            id: allocId(),
            name: params.name ?? `Sequence ${doc.project.sequences.length + 2}`,
            mark_in: null,
            mark_out: null,
            tracks: [
              { id: allocId(), kind: 'video', name: 'V1', gain_db: 0, clips: [] },
              { id: allocId(), kind: 'audio', name: 'A1', gain_db: 0, clips: [] },
            ],
          };
          doc.project.sequences.push(active);
          doc.project.sequence = sequence;
          return { created: true, sequence_id: sequence.id, name: sequence.name, activated: true };
        }
        if (id === 'sequence.duplicate') {
          const sourceId = params.sequence_id ?? active.id;
          const source = sourceId === active.id
            ? active
            : doc.project.sequences.find(sequence => sequence.id === sourceId);
          if (!source) throw new Error(`Sequence ${sourceId} not found`);
          const copy = structuredClone(source);
          copy.id = allocId();
          copy.name = params.name ?? `${source.name} copy`;
          if (sourceId === active.id) {
            doc.project.sequences.push(active);
            doc.project.sequence = copy;
          } else {
            doc.project.sequences.push(copy);
          }
          return { created: true, sequence_id: copy.id, name: copy.name, activated: sourceId === active.id };
        }
        if (id === 'sequence.rename') {
          const targetId = params.sequence_id ?? active.id;
          const target = targetId === active.id
            ? active
            : doc.project.sequences.find(sequence => sequence.id === targetId);
          if (!target) throw new Error(`Sequence ${targetId} not found`);
          target.name = params.name;
          return { renamed: true, sequence_id: targetId, name: target.name };
        }
        if (id === 'sequence.delete') {
          const targetId = params.sequence_id ?? active.id;
          if (targetId === active.id) {
            if (doc.project.sequences.length === 0) throw new Error('A project must keep one sequence');
            doc.project.sequence = doc.project.sequences.shift();
            return { deleted: true, sequence_id: targetId, active_sequence_id: doc.project.sequence.id };
          }
          doc.project.sequences = doc.project.sequences.filter(sequence => sequence.id !== targetId);
          return { deleted: true, sequence_id: targetId };
        }
        const targetId = params.sequence_id;
        const targetIndex = doc.project.sequences.findIndex(sequence => sequence.id === targetId);
        if (targetIndex < 0) throw new Error(`Sequence ${targetId} not found`);
        const next = doc.project.sequences.splice(targetIndex, 1)[0];
        doc.project.sequences.push(active);
        doc.project.sequence = next;
        return { sequence_id: next.id, name: next.name, switched: true };
      }
      if (id === 'edit.begin_gesture' || id === 'edit.end_gesture') return { gesture: true };
      if (id === 'timeline.insert_clip' || id === 'timeline.overwrite_clip') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const at = params.at;
        if (id === 'timeline.insert_clip') {
          for (const clip of track.clips) {
            if (clip.timeline_start >= at) clip.timeline_start += params.duration;
          }
        } else {
          track.clips = track.clips.filter(clip => {
            const end = clip.timeline_start + clip.duration;
            return end <= at || clip.timeline_start >= at + params.duration;
          });
        }
        track.clips.push({
          media_id: params.media_id,
          timeline_start: at,
          source_in: params.source_in,
          duration: params.duration,
          color: { exposure: 0, contrast: 1, saturation: 1 },
          effects: [],
          keyframes: [],
        });
        track.clips.sort((a, b) => a.timeline_start - b.timeline_start);
        return { track_id: track.id, clip_index: track.clips.findIndex(clip => clip.timeline_start === at), mode: id.includes('insert') ? 'insert' : 'overwrite' };
      }
      if (id === 'timeline.inspect') {
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const clip = track.clips[params.clip_index];
        return { track_id: track.id, clip_index: params.clip_index, start: clip.timeline_start, end: clip.timeline_start + clip.duration, duration: clip.duration, source_in: clip.source_in };
      }
      if (id === 'trim.inspect') {
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const clip = track.clips[params.clip_index];
        return { track_id: track.id, clip_index: params.clip_index, edge: params.edge, head_room: clip.source_in, tail_room: second * 10, cut: clip.timeline_start + clip.duration };
      }
      if (id === 'timeline.trim_clip') {
        push();
        const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
        const clip = track.clips[params.clip_index];
        const delta = params.delta;
        if (params.edge === 'start') {
          clip.timeline_start += delta;
          clip.source_in += delta;
          clip.duration -= delta;
        } else {
          clip.duration += delta;
        }
        if (params.ripple) {
          const shift = params.edge === 'start' ? -delta : delta;
          for (const later of track.clips) {
            if (later !== clip && later.timeline_start >= clip.timeline_start + clip.duration) {
              later.timeline_start += shift;
            }
          }
        }
        return { trimmed: true };
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
  for (const menu of ['File', 'Edit', 'Clip', 'Sequence', 'Markers', 'View', 'Window', 'Help']) {
    await menuBar.getByRole('button', { name: menu, exact: true }).click();
    const popover = page.locator('.menu-popover').last();
    await popover.waitFor();
    const entries = await popover.locator('button').count();
    assert.ok(entries > 0, `the ${menu} menu should list at least one command`);
    await page.keyboard.press('Escape');
    await popover.waitFor({ state: 'detached' });
  }
  // Graphics now ships the Phase 5 caption commands, so it must render; Audio
  // and Effects stay unshipped for now.
  for (const emptyMenu of ['Audio', 'Effects']) {
    assert.equal(
      await menuBar.getByRole('button', { name: emptyMenu, exact: true }).count(),
      0,
      `the ${emptyMenu} menu has no shipped commands yet and must not render`,
    );
  }
  // The tool rail exposes every editing and graphics tool in named groups.
  const toolRail = page.locator('.timeline-tool-rail');
  assert.equal(
    await toolRail.locator('.timeline-tool-button').count(),
    17,
    'the tool rail should expose all 17 tools',
  );
  assert.equal(
    await toolRail.locator('.timeline-tool-group-label').count(),
    5,
    'the tool rail should group tools into five named segments',
  );
  // Layout contract: media | toolbox | preview across the top, with a
  // full-width timeline docked at the bottom edge.
  const viewport = page.viewportSize();
  const mediaBox = await page
    .locator('.floating-panel')
    .filter({ hasText: 'Media pool' })
    .boundingBox();
  const previewBox = await page.locator('.monitor-unified').boundingBox();
  const railBox = await toolRail.boundingBox();
  const timelineBox = await page.locator('.timeline').boundingBox();
  assert.ok(viewport && mediaBox && previewBox && railBox && timelineBox);
  assert.ok(
    timelineBox.x <= 1 && timelineBox.x + timelineBox.width >= viewport.width - 1,
    'the timeline should span the app width',
  );
  assert.ok(
    railBox.x >= mediaBox.x + mediaBox.width - 2 &&
      railBox.x + railBox.width <= previewBox.x + 2,
    'the toolbox should sit between the media pool and the preview',
  );
  assert.ok(
    mediaBox.height < viewport.height * 0.75,
    'the media pool should stop above the full-width timeline',
  );

  // Sequences are created, switched, duplicated, renamed, and deleted from the
  // timeline tab strip, with the engine keeping each sequence independent.
  let sequencePrompt = 'B roll';
  page.on('dialog', async (dialog) => {
    if (dialog.type() === 'prompt') await dialog.accept(sequencePrompt);
    else await dialog.accept();
  });
  const sequenceTabs = page.locator('.sequence-tabs');
  assert.equal(
    await sequenceTabs.getByRole('tab').count(),
    1,
    'a new project starts with one sequence tab',
  );
  await sequenceTabs.getByRole('button', { name: 'New Sequence' }).click();
  const bRollTab = sequenceTabs.getByRole('tab', { name: /B roll/ });
  await bRollTab.waitFor();
  assert.equal(await bRollTab.getAttribute('aria-selected'), 'true');
  assert.equal(await sequenceTabs.getByRole('tab').count(), 2);
  await sequenceTabs.getByRole('tab', { name: /Sequence 1/ }).click();
  await sequenceTabs
    .getByRole('tab', { name: /Sequence 1/ })
    .getAttribute('aria-selected')
    .then((value) => assert.equal(value, 'true'));
  await sequenceTabs.getByRole('button', { name: 'Duplicate Active Sequence' }).click();
  await sequenceTabs.getByRole('tab', { name: /Sequence 1 copy/ }).waitFor();
  sequencePrompt = 'Main edit';
  await sequenceTabs.getByRole('button', { name: 'Rename Active Sequence' }).click();
  await sequenceTabs.getByRole('tab', { name: /Main edit/ }).waitFor();
  await sequenceTabs.getByRole('button', { name: 'Delete Active Sequence' }).click();
  await sequenceTabs.getByRole('tab', { name: /Main edit/ }).waitFor({ state: 'detached' });
  assert.equal(await sequenceTabs.getByRole('tab').count(), 2);
  await sequenceTabs.getByRole('tab', { name: /B roll/ }).click();
  const sequenceRequests = await page.evaluate(() => window.__commandRequests);
  for (const command of [
    'sequence.create',
    'project.switch_sequence',
    'sequence.duplicate',
    'sequence.rename',
    'sequence.delete',
  ]) {
    assert.ok(
      sequenceRequests.some((request) => request.id === command),
      `${command} should be dispatched by the sequence tab strip`,
    );
  }

  await toolRail.getByRole('button', { name: 'Type Tool', exact: true }).click();
  await page.locator('.status-bar-tool').getByText('Type Tool').waitFor();
  await page.locator('.app-shell').press('Shift+A');
  await page.locator('.status-bar-tool').getByText('Track Select Backward').waitFor();
  await page.locator('.app-shell').press('v');
  await page.locator('.status-bar-tool').getByText('Selection Tool').waitFor();

  // The command palette reads the same catalogue.
  await page.keyboard.press('Control+k');
  await page.locator('#command-palette-search').fill('ripple');
  const firstPaletteResult = page.locator('.command-palette-result-label').first();
  await firstPaletteResult.waitFor();
  assert.equal(
    await firstPaletteResult.textContent(),
    'Ripple Delete',
    'the palette should rank Ripple Delete first for "ripple"',
  );
  await page.keyboard.press('Escape');

  // The shortcuts editor rebinds a command and the new key dispatches it.
  await menuBar.getByRole('button', { name: 'Help', exact: true }).click();
  await page.locator('.menu-popover').getByText('Keyboard Shortcuts', { exact: true }).click();
  const shortcutsDialog = page.locator('.shortcuts-dialog');
  await shortcutsDialog.waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-shortcuts.png' });
  const undoRow = shortcutsDialog
    .locator('.shortcuts-row')
    .filter({ has: page.getByText('Undo', { exact: true }) })
    .first();
  await undoRow.getByRole('button').click();
  await page.keyboard.press('Control+Shift+U');
  await undoRow.locator('.shortcuts-key').getByText('Ctrl+Shift+U').waitFor();
  // exact: the dialog also renders "Change shortcut for Reopen Closed Panels",
  // and a non-exact match would resolve to two buttons.
  await shortcutsDialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.locator('.app-shell').press('Control+Shift+U');
  const undoRequests = await page.evaluate(() => window.__commandRequests);
  assert.ok(
    undoRequests.some((request) => request.id === 'edit.undo'),
    'a rebound shortcut should dispatch its command',
  );
  await menuBar.getByRole('button', { name: 'Help', exact: true }).click();
  await page.locator('.menu-popover').getByText('Keyboard Shortcuts', { exact: true }).click();
  await page.locator('.shortcuts-dialog').getByRole('button', { name: 'Reset all to defaults' }).click();
  await page.locator('.shortcuts-dialog').getByRole('button', { name: 'Close', exact: true }).click();

  // Preferences apply theme, glass, motion, and timeline defaults immediately.
  await page.locator('.app-shell').press('Control+,');
  const preferencesDialog = page.locator('.preferences-dialog');
  await preferencesDialog.waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-preferences.png' });
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await preferencesDialog.getByRole('button', { name: 'Light', exact: true }).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  await preferencesDialog.getByRole('button', { name: 'Dark', exact: true }).click();
  await preferencesDialog.getByRole('button', { name: 'Off', exact: true }).click();
  assert.equal(await page.locator('html').getAttribute('data-glass'), 'off');
  await preferencesDialog.getByRole('button', { name: 'Full', exact: true }).click();
  await preferencesDialog.getByRole('tab', { name: 'Timeline' }).click();
  await preferencesDialog.getByRole('button', { name: 'Compact', exact: true }).click();
  if ((await page.locator('.track-row').count()) > 0) {
    assert.ok(
      (await page.locator('.track-row.track-height-compact').count()) > 0,
      'the compact track height should reach the timeline rows',
    );
  }
  await preferencesDialog.getByRole('button', { name: 'Normal', exact: true }).click();
  await preferencesDialog.getByRole('button', { name: /Snapping on/ }).click();
  await preferencesDialog.getByRole('button', { name: /Snapping off/ }).waitFor();
  await preferencesDialog.getByRole('button', { name: /Snapping off/ }).click();
  await preferencesDialog.getByRole('button', { name: 'Close' }).click();
  const storedPreferences = await page.evaluate(() =>
    JSON.parse(localStorage.getItem('easy-edits-pro.preferences.v1') ?? '{}'),
  );
  assert.equal(storedPreferences.theme, 'dark', 'theme should persist');
  assert.equal(storedPreferences.glass, 'full', 'glass mode should persist');
  assert.equal(storedPreferences.trackHeight, 'normal', 'track height should persist');
  assert.equal(storedPreferences.snap, true, 'snapping should persist');

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

  // Layout presets retract floating panels and restore them.
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page
    .locator('.menu-popover')
    .getByRole('button', { name: 'Layout Preset: Review', exact: true })
    .click();
  await page.waitForTimeout(350);
  assert.equal(
    await page.locator('.floating-panel').count(),
    0,
    'the Review preset should hide the media and inspector panels',
  );
  await page.locator('.panel-rail').waitFor();
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page
    .locator('.menu-popover')
    .getByRole('button', { name: 'Layout Preset: Learning', exact: true })
    .click();
  await page.waitForTimeout(350);
  assert.equal(
    await page.locator('.floating-panel').count(),
    2,
    'the Learning preset should restore both panels',
  );

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
  // Clip ▸ Speed/Duration changes the clip in place and undoes as one step.
  await clip.click();
  await menuBar.getByRole('button', { name: 'Clip', exact: true }).click();
  await page.locator('.menu-popover').getByText(/Speed\/Duration/).click();
  const speedDialog = page.locator('.speed-duration-dialog');
  await speedDialog.waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-speed-duration.png' });
  await speedDialog.getByLabel('Speed percent').fill('200');
  await speedDialog.getByRole('button', { name: 'Apply', exact: true }).click();
  await page.waitForFunction(() =>
    window.__commandRequests.some(request => request.id === 'timeline.rate_stretch')
  );
  const speedRequest = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.rate_stretch').pop()
  );
  assert.equal(speedRequest.params.speed, 2, 'the dialog should send 2x speed');
  assert.equal(speedRequest.params.linked, false, 'an ungrouped clip has no linked audio');
  await page.locator('.clip-speed-badge').first().waitFor();
  assert.match(
    (await page.locator('.clip-speed-badge').first().textContent()) ?? '',
    /200%/,
    'the clip should show its new speed badge',
  );
  await page.locator('.app-shell').press('Control+z');
  await page.waitForFunction(() => !document.querySelector('.clip-speed-badge'));

  // The clip context menu is generated from the shipped Clip catalogue.
  await clip.click({ button: 'right' });
  const clipMenu = page.locator('.clip-context-menu');
  await clipMenu.waitFor();
  assert.equal(await clipMenu.getByRole('menuitem').count(), 4);
  const splitMenuItem = clipMenu.getByRole('menuitem', { name: /Split Clip at Playhead/ });
  assert.equal(
    await splitMenuItem.isDisabled(),
    true,
    'split should be disabled while the playhead is outside the clip',
  );
  assert.match(
    (await splitMenuItem.getAttribute('title')) ?? '',
    /outside this clip/,
    'the disabled split should explain why',
  );
  await clipMenu.getByRole('menuitem', { name: /Speed\/Duration/ }).click();
  const contextSpeedDialog = page.locator('.speed-duration-dialog');
  await contextSpeedDialog.waitFor();
  await contextSpeedDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await clip.click({ button: 'right' });
  await page.locator('.clip-context-menu').getByRole('menuitem', { name: /Remove Clip/ }).click();
  await page.waitForFunction(() => document.querySelectorAll('.timeline-clip').length === 0);
  await page.locator('.app-shell').press('Control+z');
  await page.locator('.timeline-clip').first().waitFor();

  // Every tool stamps its cursor class onto the clips it can act on.
  await page.locator('.timeline-tool-rail').getByRole('button', { name: 'Razor Tool', exact: true }).click();
  assert.ok(
    (await page.locator('.timeline-clip.tool-razor').count()) > 0,
    'the razor tool should set the razor cursor class on clips',
  );
  // Razor splits at the pointer, not just the playhead.
  const clipBox = await clip.boundingBox();
  await page.mouse.click(clipBox.x + clipBox.width * 0.5, clipBox.y + clipBox.height * 0.5);
  const razorRequests = await page.evaluate(() => window.__commandRequests);
  const razorSplit = razorRequests.filter((request) => request.id === 'timeline.split_clip').pop();
  assert.ok(
    razorSplit && typeof razorSplit.params?.at === 'number',
    'the razor tool should split at a numeric tick under the pointer',
  );
  assert.equal(
    await page.locator('.timeline-clip').count(),
    2,
    'the razor tool should split the clicked clip in two',
  );
  await page.locator('.app-shell').press('Control+z');
  await page.locator('.timeline-clip').first().waitFor();
  assert.equal(await page.locator('.timeline-clip').count(), 1, 'undo should restore the split clip');

  // Track select forward selects the clicked clip and everything after it.
  await page
    .locator('.timeline-tool-rail')
    .getByRole('button', { name: 'Track Select Forward', exact: true })
    .click();
  const restoredBox = await clip.boundingBox();
  await page.mouse.click(
    restoredBox.x + restoredBox.width * 0.5,
    restoredBox.y + restoredBox.height / 2,
  );
  await page.locator('.timeline-clip.tool-trackForward.selected').first().waitFor();
  // Phase 2: the Ripple tool's tail drag trims through the engine, opens the
  // Trim Monitor, and commits as a single undoable gesture.
  await page.locator('.timeline-tool-rail').getByRole('button', { name: 'Ripple Edit Tool', exact: true }).click();
  const widthBeforeTrim = await clip.evaluate(element => element.getBoundingClientRect().width);
  const trimHandle = clip.locator('.clip-trim-right');
  const handleBounds = await trimHandle.boundingBox();
  await page.mouse.move(handleBounds.x + handleBounds.width / 2, handleBounds.y + handleBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(
    handleBounds.x + handleBounds.width / 2 - 48,
    handleBounds.y + handleBounds.height / 2,
    { steps: 8 },
  );
  await page.locator('.trim-monitor').waitFor();
  await page.mouse.up();
  await page.waitForFunction(() =>
    window.__commandRequests.some(request => request.id === 'timeline.trim_clip')
  );
  const trimRequest = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').pop()
  );
  assert.equal(trimRequest.params.edge, 'end', 'the right handle should trim the tail');
  assert.equal(trimRequest.params.ripple, true, 'the Ripple tool should request a ripple trim');
  assert.ok(trimRequest.params.delta < 0, 'dragging left should shorten the tail');
  await page.waitForFunction(
    width => document.querySelector('.timeline-clip')?.getBoundingClientRect().width < width - 1,
    widthBeforeTrim,
  );
  await page.locator('.app-shell').press('Control+z');
  await page.waitForFunction(
    width => Math.abs((document.querySelector('.timeline-clip')?.getBoundingClientRect().width ?? 0) - width) <= 1,
    widthBeforeTrim,
  );

  // Keyboard Trim Mode: enter on the selected clip, shuttle back with J, stop
  // with K, then commit one undoable trim with Enter.
  const trimsBeforeDynamic = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').length
  );
  await page.locator('.app-shell').press('Control+Alt+E');
  await page.locator('.trim-monitor-actions').waitFor();
  await page.locator('.app-shell').press('j');
  await page.waitForTimeout(250);
  await page.locator('.app-shell').press('k');
  await page.locator('.app-shell').press('Enter');
  await page.waitForFunction(
    count => window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').length > count,
    trimsBeforeDynamic,
  );
  const dynamicTrim = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').pop()
  );
  assert.ok(dynamicTrim.params.delta < 0, 'J should shuttle the trim backwards');
  await page.locator('.app-shell').press('Control+z');
  await page.waitForFunction(
    width => Math.abs((document.querySelector('.timeline-clip')?.getBoundingClientRect().width ?? 0) - width) <= 1,
    widthBeforeTrim,
  );

  await page.locator('.timeline-tool-rail').getByRole('button', { name: 'Selection Tool', exact: true }).click();
  // Three-point edit from the Source monitor: click the clip to focus the
  // source tab, then send its marked range to the playhead.
  await page.mouse.click(
    restoredBox.x + restoredBox.width * 0.5,
    restoredBox.y + restoredBox.height / 2,
  );
  await page.locator('.source-edit-bar').waitFor();
  await page.locator('.source-edit-bar').getByRole('button', { name: 'Insert source range', exact: true }).click();
  await page.waitForFunction(() =>
    window.__commandRequests.some(request => request.id === 'timeline.insert_clip')
  );
  const insertRequest = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.insert_clip').pop()
  );
  assert.equal(typeof insertRequest.params.media_id, 'number', 'insert should carry a media id');
  assert.ok(insertRequest.params.duration > 0, 'insert should carry a positive source duration');
  await page.locator('.app-shell').press('Control+z');

  // Clicking a clip focuses the Source tab; the playhead checks below want Program.
  await page
    .locator('.monitor-unified .panel-tabs')
    .getByText('Program Timeline', { exact: true })
    .click();
  // Catalogue-bound playhead commands: End jumps to the sequence end, Home
  // returns to zero, and the transport timecode follows both.
  await page.locator('.app-shell').press('End');
  await page.waitForFunction(
    () => document.querySelector('.transport code')?.textContent !== '00:00:00'
  );
  await page.locator('.app-shell').press('Home');
  await page.waitForFunction(
    () => document.querySelector('.transport code')?.textContent === '00:00:00'
  );
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

  // Phase 4 slice 2: bins and asset metadata. Bins used to call command ids
  // that did not exist, so this asserts the whole create→rename→assign→delete
  // path reaches the engine.
  await page.locator('[data-tooltip="Create new media bin / folder"]').click();
  const binModal = page.locator('.modal-dialog-card').filter({ hasText: 'Create New Media Bin' });
  await binModal.locator('input[type="text"]').fill('Selects');
  await binModal.getByRole('button', { name: 'Create Bin' }).click();
  await page.locator('.media-bin-pill', { hasText: 'Selects' }).waitFor();

  sequencePrompt = 'Selects 4K';
  await page.locator('.bin-rename-btn').first().click();
  await page.waitForFunction(() =>
    document.querySelector('.media-bins-strip')?.textContent?.includes('Selects 4K'),
  );

  await page.locator('.media-edit-meta-btn').first().click();
  const propsModal = page.locator('.modal-dialog-card').filter({ hasText: 'Edit Asset Properties' });
  await propsModal.locator('input[placeholder="12B"]').fill('12B');
  await propsModal.locator('input[placeholder="3"]').fill('4');
  await propsModal
    .locator('textarea')
    .fill('Checked the rooftop audio; clean take.');
  await propsModal.locator('select').selectOption({ label: '📁 Selects 4K' });
  await propsModal.getByRole('button', { name: 'Save Properties' }).click();
  await page.waitForFunction(() =>
    document.querySelector('.media-list')?.textContent?.includes('Selects 4K'),
  );

  const mediaOrganisation = await page.evaluate(() => ({
    binCreate: window.__commandRequests.some(request => request.id === 'media.create_bin'),
    binRename: window.__commandRequests.some(request => request.id === 'media.rename_bin'),
    binMove: window.__commandRequests.some(request => request.id === 'media.move_to_bin'),
    properties: window.__commandRequests.filter(request => request.id === 'media.update_properties').at(-1)?.params ?? null,
  }));
  assert.equal(mediaOrganisation.binCreate, true, 'creating a bin must reach media.create_bin');
  assert.equal(mediaOrganisation.binRename, true, 'renaming a bin must reach media.rename_bin');
  assert.equal(mediaOrganisation.binMove, true, 'assigning media must reach media.move_to_bin');
  assert.equal(mediaOrganisation.properties?.scene, '12B', 'scene metadata should be sent');
  assert.equal(mediaOrganisation.properties?.take, '4', 'take metadata should be sent');
  assert.match(mediaOrganisation.properties?.comment ?? '', /rooftop audio/);

  await page.locator('.bin-close-btn').first().click();
  await page.waitForFunction(() =>
    !document.querySelector('.media-bins-strip')?.textContent?.includes('Selects 4K'),
  );
  assert.equal(
    await page.evaluate(() => window.__commandRequests.some(request => request.id === 'media.delete_bin')),
    true,
    'deleting a bin must reach media.delete_bin',
  );

  // Phase 4 slice 1: markers. M adds at the playhead, Shift+M marks a range,
  // the lane renders both, Shift+Right jumps to the next marker, and the
  // Markers panel lists them without leaving the editor.
  await page.locator('.timeline-ruler').click({ position: { x: 320, y: 12 } });
  await page.keyboard.press('m');
  await page.locator('.marker-pin').first().waitFor();
  assert.equal(await page.locator('.marker-pin').count(), 1, 'M should add one marker');
  await page.keyboard.press('Shift+M');
  await page.locator('.marker-pin.is-range').waitFor();
  assert.equal(await page.locator('.marker-pin').count(), 2, 'Shift+M should add a range marker');
  assert.equal(await page.locator('.marker-pin.is-range').count(), 1, 'range markers render as bars');
  await page.keyboard.press('Shift+ArrowRight');
  await page.keyboard.press('ControlOrMeta+k');
  await page.locator('.command-palette').waitFor();
  await page.locator('#command-palette-search').fill('markers panel');
  await page.locator('.command-palette-result', { hasText: 'Markers Panel' }).first().click();
  await page.locator('.markers-panel').waitFor();
  assert.equal(await page.locator('.marker-row').count(), 2, 'the panel lists every marker');
  const markerRequests = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id.startsWith('marker.')).map(request => request.id),
  );
  assert.ok(markerRequests.includes('marker.add'), 'marker.add should reach the engine');
  assert.ok(markerRequests.includes('marker.add_range'), 'marker.add_range should reach the engine');
  assert.ok(markerRequests.includes('marker.next'), 'Shift+Right should dispatch marker.next');
  const markerCount = await page.locator('.marker-row').count();
  await page.locator('.marker-row .marker-delete').first().click();
  await page.waitForFunction(() => document.querySelectorAll('.marker-row').length === 1);
  assert.equal(await page.locator('.marker-row').count(), 1, 'deleting from the panel removes the marker');

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
  console.log(JSON.stringify({ page: await page.url(), viewport: page.viewportSize(), welcome: 'passed', importWorkspace: 'passed', importMediaSelection: 'passed', audioWorkspace: 'passed', audioDspCommand: audioCommand, exportWorkspace: 'passed', hardwareEncoderFormatToggle: 'passed', mediaPointerDropPx: initialLeft, zoomWidth: { before: widthBefore, after: widthAfter }, scrubSeekSeconds: seekTime, playbackAdvancedToSeconds: playingTime, nextFrameSeconds: nextFrameTime, previousFrameSeconds: previousFrameTime, movedClipLeftPx: movedLeft, markerRequests, markerCount, mediaOrganisation, reducedMotion, editorScreenshot: '/tmp/easy-edits-pro-editor-smoke.png', importScreenshot: '/tmp/easy-edits-pro-import-workspace.png', exportScreenshot: '/tmp/easy-edits-pro-export-workspace.png', browserErrors: pageErrors.length }, null, 2));
} finally {
  await browser.close();
  server.kill('SIGTERM');
}
