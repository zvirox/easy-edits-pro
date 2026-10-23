// In-page mock of the Tauri backend for the smoke suite. This intercepts
// every `invoke` and answers from an in-memory project document, so the
// suite exercises the UI without the real engine (see tests/e2e/README.md).
export function installMockBackend(page, { mediaUrl, catalogue }) {
  return page.addInitScript(setup, { mediaUrl, catalogue });
}

function setup({ mediaUrl, catalogue }) {
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
}
