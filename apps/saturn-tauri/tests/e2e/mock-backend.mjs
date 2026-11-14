// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
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
  let nextChangeId = 1;
  let headRevision = 1;
  let activeBranch = 'main';
  const branches = new Map();
  const undoStack = [];
  const history = [];
  const agentSessions = new Map();
  const eventListeners = new Map();
  window.__commandRequests = [];
  window.__mockHistory = history;
  window.__mockBranches = branches;
  const snapshot = () => ({ document: structuredClone(doc), path: currentPath, is_dirty: undoStack.length > 0 });
  const push = () => undoStack.push(structuredClone(doc));
  const emitEvent = (eventName, payload = {}) => {
    for (const cb of eventListeners.get(eventName) ?? []) {
      try { cb({ event: eventName, payload }); } catch { /* listener cleanup */ }
    }
  };
  const clipRange = (clipIds, project) => {
    let min = null;
    let max = null;
    for (const track of project?.sequence?.tracks ?? []) {
      for (const clip of track.clips ?? []) {
        if (clipIds.includes(clip.id)) {
          min = min == null ? clip.timeline_start : Math.min(min, clip.timeline_start);
          max = max == null ? clip.timeline_start + clip.duration : Math.max(max, clip.timeline_start + clip.duration);
        }
      }
    }
    return min != null && max != null ? [min, max] : [0, 0];
  };
  const checkUndoConflict = changeId => {
    const idx = history.findIndex(r => r.change_id === changeId);
    if (idx === -1) return `Change #${changeId} not found`;
    const target = history[idx];
    if (target.undone) return `Change #${changeId} was already undone`;
    for (let i = idx + 1; i < history.length; i += 1) {
      const later = history[i];
      if (later.undone || later.branch !== target.branch) continue;
      const overlap = later.affected_entity_ids.some(eid => target.affected_entity_ids.includes(eid));
      if (overlap) {
        return `Conflict: later ${later.actor} edit (${later.command_id}) touched overlapping clip(s)`;
      }
    }
    return null;
  };
  const findAllClipIds = project => {
    const ids = new Map();
    for (const track of project?.sequence?.tracks ?? []) {
      for (const clip of track.clips ?? []) {
        if (clip.id != null) ids.set(clip.id, JSON.stringify(clip));
      }
    }
    return ids;
  };
  const recordChange = (actor, sessionId, commandId, beforeDoc, rationale = '', confidence = 'medium') => {
    const beforeClips = findAllClipIds(beforeDoc?.project);
    const afterClips = findAllClipIds(doc?.project);
    const affected = [];
    for (const [cid, json] of afterClips.entries()) {
      if (beforeClips.get(cid) !== json) affected.push(cid);
    }
    for (const cid of beforeClips.keys()) {
      if (!afterClips.has(cid) && !affected.includes(cid)) affected.push(cid);
    }
    if (actor.startsWith('agent:') && affected.length > 0) {
      for (const track of doc?.project?.sequence?.tracks ?? []) {
        for (const clip of track.clips ?? []) {
          if (affected.includes(clip.id)) {
            clip.agent_note = {
              rationale: rationale || `${commandId}`,
              confidence,
              actor: { kind: 'agent', id: actor.slice(6) },
              session_id: sessionId,
              change_id: nextChangeId,
              active: true,
            };
          }
        }
      }
    }
    headRevision += 1;
    history.push({
      change_id: nextChangeId++,
      transaction_id: headRevision,
      result_revision: headRevision,
      actor,
      is_human: actor === 'human',
      session_id: sessionId,
      branch: activeBranch,
      command_id: commandId,
      tier: commandId.startsWith('review.') || commandId.startsWith('tasksheet.') || commandId.startsWith('project.') ? 'edit' : 'edit',
      affected_entity_ids: affected,
      tick_range: clipRange(affected, doc?.project ?? beforeDoc?.project),
      rationale: rationale || null,
      confidence,
      inverse_state: structuredClone(beforeDoc),
      undone: false,
    });
  };
  const execute = (request, ctx = { actor: 'human', sessionId: 'human-ui' }) => {
    const { id, params = {} } = request;
    window.__commandRequests.push({ id, params, actor: ctx.actor });
    if (ctx.actor.startsWith('agent:')) {
      const sess = agentSessions.get(ctx.sessionId);
      if (sess?.is_stopped) {
        throw new Error(`SessionStopped: agent session ${ctx.sessionId} is stopped`);
      }
      if (!sess) {
        agentSessions.set(ctx.sessionId, {
          session_id: ctx.sessionId,
          actor: { kind: 'agent', id: ctx.actor.slice(6) },
          is_stopped: false,
          generation: 1,
          active_branch: activeBranch,
          playhead: 0,
        });
      } else {
        sess.active_branch = activeBranch;
      }
    }
    if (id === 'project.status') return snapshot();
    if (id === 'project.new') {
      doc = { format: 'project-saturn.project', schema_version: 5, generator: 'browser-smoke-test', project: {
        name: params.name ?? 'Browser Test', settings: { width: params.width ?? 1920, height: params.height ?? 1080, frame_rate: params.frame_rate ?? { numerator: 30, denominator: 1 }, audio_sample_rate: 48000 },
        media: [], next_id: 3, sequences: [], brief: { goal: '', platform: null, target_duration_ticks: null, duration_tolerance_ticks: null, must_keep_clip_ids: [], banned_content: [], style_notes: '', loudness_target_lufs: null, source: 'user_prompt' },
        task_sheet: null,
        sequence: { id: 1, name: params.sequence_name ?? 'Sequence 1', mark_in: null, mark_out: null, tracks: [
          { id: 1, kind: 'video', name: 'V1', gain_db: 0, clips: [] }, { id: 2, kind: 'audio', name: 'A1', gain_db: 0, clips: [] },
        ] },
      }, view: {} };
      currentPath = null;
      undoStack.length = 0;
      history.length = 0;
      branches.clear();
      agentSessions.clear();
      activeBranch = 'main';
      headRevision = 1;
      return { created: true };
    }
    if (id === 'project.branch_create') {
      const branchName = params.name;
      if (!branchName || branches.has(branchName)) throw new Error(`Invalid or duplicate branch: ${branchName}`);
      branches.set(branchName, { name: branchName, parent: activeBranch, base_revision: headRevision, note: params.note ?? null, created_at_unix_ms: Date.now(), state: structuredClone(doc.project) });
      return { created: true, branch: branchName };
    }
    if (id === 'project.branch_list') {
      return {
        active: activeBranch,
        branches: [...branches.values()].map(b => ({ name: b.name, note: b.note, parent: b.parent, created_at_unix_ms: b.created_at_unix_ms })),
      };
    }
    if (id === 'project.branch_switch') {
      const target = params.name;
      if (activeBranch !== 'main' && branches.has(activeBranch)) {
        branches.get(activeBranch).state = structuredClone(doc.project);
      } else if (activeBranch === 'main') {
        branches.set('__main__', { name: 'main', state: structuredClone(doc.project) });
      }
      if (target === 'main') {
        if (branches.has('__main__')) doc.project = structuredClone(branches.get('__main__').state);
        activeBranch = 'main';
      } else {
        const b = branches.get(target);
        if (!b) throw new Error(`Branch ${target} does not exist`);
        doc.project = structuredClone(b.state);
        activeBranch = target;
      }
      for (const s of agentSessions.values()) {
        if (s.session_id === ctx.sessionId) s.active_branch = activeBranch;
      }
      return { switched: true, active: activeBranch };
    }
    if (id === 'project.branch_merge') {
      const branchName = params.branch;
      if (branchName === activeBranch) throw new Error(`Branch ${branchName} is the active branch; switch away before merging`);
      const target = branches.get(branchName);
      if (!target) throw new Error(`Branch ${branchName} does not exist`);
      if (params.action === 'accept') {
        doc.project = structuredClone(target.state);
      }
      branches.delete(branchName);
      for (const s of agentSessions.values()) {
        if (s.active_branch === branchName) s.active_branch = activeBranch;
      }
      history.push({
        change_id: nextChangeId++,
        transaction_id: ++headRevision,
        result_revision: headRevision,
        actor: ctx.actor,
        is_human: ctx.actor === 'human',
        session_id: ctx.sessionId,
        branch: activeBranch,
        command_id: 'project.branch_merge',
        tier: 'edit',
        affected_entity_ids: [],
        tick_range: [0, 0],
        rationale: `${params.action} branch ${branchName}`,
        confidence: 'high',
        inverse_state: null,
        undone: false,
      });
      return { merged: params.action === 'accept', discarded: params.action === 'reject', branch: branchName };
    }
    if (id === 'project.brief_get') {
      return structuredClone(doc.project.brief ?? {});
    }
    if (id === 'project.brief_set') {
      doc.project.brief = { ...doc.project.brief, ...params };
      return { updated: true, brief: structuredClone(doc.project.brief) };
    }
    if (id === 'tasksheet.create') {
      doc.project.task_sheet = structuredClone({ schema: 'tasksheet/1', rounds: [], questions: [], ...params });
      return { created: true, sheet: structuredClone(doc.project.task_sheet) };
    }
    if (id === 'tasksheet.update') {
      doc.project.task_sheet = { ...(doc.project.task_sheet ?? { schema: 'tasksheet/1', id: 'ts_default', title: 'Task Sheet', requester: 'human:user', deliverables: [], status: 'draft', rounds: [], questions: [] }), ...params };
      return { updated: true, sheet: structuredClone(doc.project.task_sheet) };
    }
    if (id === 'tasksheet.submit') {
      if (!doc.project.task_sheet) throw new Error('No task sheet attached');
      doc.project.task_sheet.status = 'ready';
      return { submitted: true, status: 'ready', sheet: structuredClone(doc.project.task_sheet) };
    }
    if (id === 'tasksheet.get') {
      return { sheet: structuredClone(doc.project.task_sheet ?? { schema: 'tasksheet/1', id: 'ts_default', title: 'Default Project Sheet', requester: 'human:user', inputs: [], deliverables: [], status: 'draft', rounds: [], questions: [] }), status: doc.project.task_sheet?.status ?? 'draft' };
    }
    if (id === 'tasksheet.question_answer') {
      const sheet = doc.project.task_sheet;
      if (!sheet) throw new Error('No task sheet attached');
      sheet.questions = (sheet.questions ?? []).map(q => q.id === params.question_id ? { ...q, status: 'answered', selected_option_id: params.option_id } : q);
      if (sheet.status === 'needs_input') sheet.status = 'running';
      return { answered: true, sheet: structuredClone(sheet) };
    }
    if (id === 'review.list_changes') {
      const filtered = history
        .filter(r => !params.branch || r.branch === params.branch)
        .filter(r => !['project.branch_switch', 'project.branch_create', 'project.branch_merge'].includes(r.command_id))
        .map(r => {
          const reason = checkUndoConflict(r.change_id);
          return {
            change_id: r.change_id,
            transaction_id: r.transaction_id,
            actor: r.actor,
            is_human: r.is_human,
            session_id: r.session_id,
            branch: r.branch,
            command_id: r.command_id,
            tier: r.tier,
            tick_range: r.tick_range,
            affected_entity_ids: r.affected_entity_ids,
            rationale: r.rationale,
            confidence: r.confidence,
            conflict: !r.undone && reason != null,
            conflict_reason: r.undone ? null : reason,
            undone: r.undone,
          };
        })
        .reverse();
      return { branch: params.branch ?? activeBranch, changes: filtered, count: filtered.length };
    }
    if (id === 'review.round_open' || id === 'review.round_close' || id === 'review.note_add' || id === 'review.note_resolve') {
      doc.project.task_sheet ??= { schema: 'tasksheet/1', id: 'ts_default', title: 'Default Project Sheet', requester: 'human:user', inputs: [], deliverables: [], status: 'in_review', rounds: [], questions: [] };
      const sheet = doc.project.task_sheet;
      sheet.rounds ??= [];
      if (id === 'review.round_open') {
        if (sheet.rounds.some(r => r.completed_at_unix_ms == null)) throw new Error('Close open review round before opening a new one');
        const nextNum = sheet.rounds.reduce((m, r) => Math.max(m, r.round_number), 0) + 1;
        const round = { round_number: nextNum, label: params.label || `v${nextNum}`, started_at_unix_ms: Date.now(), opened_by: ctx.actor, notes: [], diff_from_prior: params.diff_from_prior ?? null };
        sheet.rounds.push(round);
        sheet.status = 'in_review';
        return { round, sheet: structuredClone(sheet) };
      }
      if (id === 'review.round_close') {
        const open = [...sheet.rounds].reverse().find(r => r.completed_at_unix_ms == null);
        if (!open) throw new Error('No open review round to close');
        const openNotes = sheet.rounds.flatMap(r => r.notes ?? []).filter(n => n.status === 'open');
        if (params.outcome === 'approved' && openNotes.length > 0) throw new Error('Cannot approve with open review notes');
        open.completed_at_unix_ms = Date.now();
        open.closed_by = ctx.actor;
        open.outcome = params.outcome;
        sheet.status = params.outcome === 'approved' ? 'approved' : 'revision_requested';
        return { round: open, sheet: structuredClone(sheet) };
      }
      if (id === 'review.note_add') {
        let round = params.round_number != null ? sheet.rounds.find(r => r.round_number === params.round_number) : sheet.rounds.find(r => r.completed_at_unix_ms == null);
        if (!round) {
          const nextNum = sheet.rounds.reduce((m, r) => Math.max(m, r.round_number), 0) + 1;
          round = { round_number: nextNum, label: `v${nextNum}`, started_at_unix_ms: Date.now(), opened_by: ctx.actor, notes: [] };
          sheet.rounds.push(round);
        }
        const note = { id: `note_r${round.round_number}_${(round.notes?.length ?? 0) + 1}`, author: ctx.actor, created_at_unix_ms: Date.now(), text: params.text, target_tick: params.target_tick, end_tick: params.end_tick, status: 'open' };
        round.notes.push(note);
        sheet.status = 'in_review';
        return { note, round_number: round.round_number, sheet: structuredClone(sheet) };
      }
      if (id === 'review.note_resolve') {
        for (const r of sheet.rounds) {
          const note = (r.notes ?? []).find(n => n.id === params.note_id);
          if (note) {
            note.status = params.status ?? 'resolved';
            note.resolution = params.resolution;
            note.resolved_by = ctx.actor;
            note.resolved_at_unix_ms = Date.now();
            return { note, sheet: structuredClone(sheet) };
          }
        }
        throw new Error(`Review note ${params.note_id} not found`);
      }
    }
    if (id === 'verify.run') {
      return { preset: params.preset ?? 'web-1080p', checks: [], summary: { total_checks: 0, passed: 0, warned: 0, failed: 0, unavailable: 0 }, notes: { aggregate_score: null, message: 'Measured evidence only' } };
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
      const before = structuredClone(doc);
      push();
      const track = doc.project.sequence.tracks.find(item => item.id === params.track_id);
      const media = doc.project.media.find(item => item.id === params.media_id);
      const start = params.timeline_start ?? track.clips.reduce((end, clip) => Math.max(end, clip.timeline_start + clip.duration), 0);
      const clipId = (doc.project.next_id = (doc.project.next_id ?? 10) + 1);
      track.clips.push({ id: clipId, media_id: media.id, timeline_start: start, source_in: 0, duration: params.duration ?? media.duration ?? 6 * second, color: { exposure: 0, contrast: 1, saturation: 1 }, effects: [], keyframes: [] });
      track.clips.sort((a, b) => a.timeline_start - b.timeline_start);
      recordChange(ctx.actor, ctx.sessionId, id, before, params.rationale ?? '', params.confidence ?? 'medium');
      return { track_id: track.id, clip_id: clipId, clip_index: track.clips.findIndex(clip => clip.id === clipId) };
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
    if (id === 'timeline.asymmetric_trim') {
      throw new Error('Split-edit preparation is unavailable in the browser mock');
    }
    if (id === 'edit.match_cut_align') {
      throw new Error('Match-cut analysis is unavailable in the browser mock');
    }
    if (id === 'timeline.beat_grid_quantize') {
      throw new Error('Beat-grid analysis is unavailable in the browser mock');
    }
    if (id === 'timeline.trim_clip') {
      const before = structuredClone(doc);
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
      recordChange(ctx.actor, ctx.sessionId, id, before, params.rationale ?? '', params.confidence ?? 'medium');
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
    if (id === 'export.presets') {
      return {
        presets: {
          source: { format: 'mp4', resolution: 'source', frame_rate: 'source' },
          youtube_1080p: { format: 'mp4', resolution: '1920x1080', frame_rate: 'source' },
          tiktok_1080x1920: { format: 'mp4', resolution: '1080x1920', frame_rate: 'source' },
          instagram_square: { format: 'mp4', resolution: '1080x1080', frame_rate: 'source' },
        },
      };
    }
    if (id === 'export.resolve') {
      const pSettings = doc?.project?.settings ?? { width: 1920, height: 1080, frame_rate: { numerator: 30, denominator: 1 } };
      let width = pSettings.width;
      let height = pSettings.height;
      if (params?.resolution && params.resolution !== 'source') {
        const parts = params.resolution.split('x').map(Number);
        if (parts[0] && parts[1]) {
          width = parts[0];
          height = parts[1];
        }
      }
      let frame_rate_numerator = pSettings.frame_rate?.numerator ?? 30;
      let frame_rate_denominator = pSettings.frame_rate?.denominator ?? 1;
      if (params?.frame_rate && params.frame_rate !== 'source') {
        const parts = params.frame_rate.split('/').map(Number);
        if (parts[0] && parts[1]) {
          frame_rate_numerator = parts[0];
          frame_rate_denominator = parts[1];
        }
      }
      const format = params?.format || 'mp4';
      const use_hardware = params?.use_hardware ?? (format === 'mp4');
      const use_mark_range = Boolean(params?.use_mark_range);
      return {
        format,
        width,
        height,
        frame_rate_numerator,
        frame_rate_denominator,
        use_mark_range,
        use_hardware,
        range: use_mark_range ? { start: doc?.project?.sequence?.mark_in ?? 0, end: doc?.project?.sequence?.mark_out ?? 1000 } : null,
      };
    }
    if (id === 'edit.undo') {
      if (undoStack.length) doc = undoStack.pop();
      return { undone: true };
    }
    return { ok: true };
  };
  window.__mockEmitProjectChanged = () => emitEvent('mcp_project_changed', {});
  window.__mockAgentExecute = (sessionId, actorId, request) => {
    const out = execute(request, { actor: `agent:${actorId}`, sessionId });
    emitEvent('mcp_project_changed', { session_id: sessionId, actor: actorId });
    return out;
  };
  let nextListenerId = 1;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' } },
    transformCallback: cb => {
      const id = nextListenerId++;
      window.__tauriCallbacks ??= new Map();
      window.__tauriCallbacks.set(id, cb);
      return id;
    },
    unregisterCallback: () => {},
    convertFileSrc: () => mediaUrl,
    invoke: async (command, args = {}) => {
      if (command === 'recovery_status') return { available: false };
      if (command === 'register_write_grant') return null;
      if (command === 'get_command_catalogue') return catalogue;
      if (command === 'command_status') return { enabled: args.id === 'edit.undo', reason: null };
      if (command === 'check_media_availability') return (doc?.project.media ?? []).map(media => ({ mediaId: media.id, missing: false }));
      if (command === 'prepare_video_preview') return '/mock/editor-smoke-preview.webm';
      if (command === 'execute_command') return execute(args.request);
      if (command === 'agent_session_status') {
        const all = [...agentSessions.values()];
        const target = args.sessionId ? agentSessions.get(args.sessionId) : (all.at(-1) ?? null);
        if (!target) {
          return { has_active_session: false, session_id: null, is_stopped: false, generation: 0, actor: null, active_branch: activeBranch, playhead: 0, locks: [], sessions: [] };
        }
        return { ...target, has_active_session: true, locks: [], sessions: all };
      }
      if (command === 'agent_session_stop') {
        const sess = agentSessions.get(args.sessionId);
        if (!sess) throw new Error(`Unknown session: ${args.sessionId}`);
        sess.is_stopped = true;
        sess.generation += 1;
        return { session_id: sess.session_id, stopped: true, generation: sess.generation };
      }
      if (command === 'agent_session_resume') {
        const sess = agentSessions.get(args.sessionId);
        if (!sess) throw new Error(`Unknown session: ${args.sessionId}`);
        sess.is_stopped = false;
        sess.generation += 1;
        return { session_id: sess.session_id, resumed: true, generation: sess.generation };
      }
      if (command === 'agent_undo_change') {
        const reason = checkUndoConflict(args.changeId);
        if (reason) throw new Error(reason);
        const rec = history.find(r => r.change_id === args.changeId);
        const targetIds = rec.affected_entity_ids;
        for (const track of doc.project.sequence.tracks) {
          track.clips = track.clips.filter(c => !targetIds.includes(c.id));
        }
        for (const rtrack of rec.inverse_state.project.sequence.tracks) {
          const restored = rtrack.clips.filter(c => targetIds.includes(c.id));
          if (restored.length) {
            const dst = doc.project.sequence.tracks.find(t => t.id === rtrack.id);
            dst?.clips.push(...structuredClone(restored));
            dst?.clips.sort((a, b) => a.timeline_start - b.timeline_start);
          }
        }
        rec.undone = true;
        return { undone_change_id: rec.change_id, undo_change_id: nextChangeId++, command_id: `undo:${rec.command_id}`, affected_entities: targetIds };
      }
      if (command === 'import_media') {
        for (const mediaPath of args.paths ?? []) doc.project.media.push({ id: nextId++, name: mediaPath.split('/').at(-1), path: mediaPath, kind: 'video', duration: 6 * second, width: 640, height: 360 });
        return snapshot();
      }
      if (command === 'get_media_server_port') return 1421;
      if (command === 'get_media_server_token') return 'browser-smoke-token';
      if (command === 'get_all_jobs') return [];
      if (command === 'plugin:event|listen') {
        const list = eventListeners.get(args.event) ?? [];
        const cb = window.__tauriCallbacks?.get(args.handler);
        if (cb) list.push(cb);
        eventListeners.set(args.event, list);
        return list.length;
      }
      if (command === 'plugin:event|unlisten') return null;
      if (command === 'plugin:dialog|save') return '/tmp/browser-test.saturn';
      if (command === 'plugin:dialog|open') return ['/mock/editor-smoke.webm'];
      if (command.startsWith('plugin:window|')) return null;
      throw new Error(`Unhandled mocked Tauri command: ${command}`);
    },
  };
}
