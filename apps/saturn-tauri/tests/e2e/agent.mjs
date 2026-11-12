// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
// End-to-end verification of AF6.1:
// 1. Default app state has NO seeded fake agent session, branch, sheet, or queue.
// 2. Scripted agent edits on a branch -> Review UI lists real queue -> Stop halts <=1s
//    and fences late agent calls -> single-change reject reverts via engine -> later
//    human edit blocks conflicting revert -> Accept All switches to main & merges ->
//    history preserves actors -> ?demo=1 loads demo seed only behind flag.
import assert from 'node:assert/strict';
import { installMockBackend } from './mock-backend.mjs';

export async function run(ctx) {
  const page = ctx.page;
  const rightDock = page.locator('.shell-dock-right');
  const statusChip = page.locator('.agent-status-chip');

  // 1. Assert default startup state exposes no fake session or queue.
  await statusChip.getByText('No session').waitFor();

  const agentTab = rightDock.getByRole('tab', { name: 'Agent', exact: true });
  await agentTab.click();
  const agentCard = page.locator('.agent-card');
  await agentCard.waitFor();
  await agentCard.getByText('No Active Session').waitFor();
  await agentCard.getByText('No project brief configured yet.').waitFor();

  const reviewTab = rightDock.getByRole('tab', { name: 'Review', exact: true });
  await reviewTab.click();
  const reviewCard = page.locator('.review-card');
  await reviewCard.waitFor();
  await reviewCard.getByText('Branch Review: main').waitFor();
  await reviewCard.getByText('No proposed agent changes yet.').waitFor();
  assert.equal(await reviewCard.getByRole('button', { name: 'Accept All', exact: true }).isDisabled(), true);

  // 2. Scripted agent creates branch `agent/cut-1` and performs 2 timeline edits.
  await page.evaluate(() => {
    const sec = 254016000000;
    const status = window.__commandRequests.findLast(r => r.id === 'project.status');
    void status;
    window.__mockAgentExecute('sess-e2e-1', 'worker', {
      id: 'project.branch_create',
      params: { name: 'agent/cut-1', note: 'Rough cut proposal' },
    });
    window.__mockAgentExecute('sess-e2e-1', 'worker', {
      id: 'project.branch_switch',
      params: { name: 'agent/cut-1' },
    });
  });
  await page.evaluate(async () => {
    const sec = 254016000000;
    const status = await window.__TAURI_INTERNALS__.invoke('execute_command', {
      request: { id: 'project.status', params: {} },
    });
    const mediaId = status.document.project.media[0]?.id;
    window.__mockAgentExecute('sess-e2e-1', 'worker', {
      id: 'timeline.add_clip',
      params: {
        track_id: 1,
        media_id: mediaId,
        timeline_start: 8 * sec,
        duration: 4 * sec,
        rationale: 'Cold open hook clip',
        confidence: 'low',
      },
    });
    window.__mockAgentExecute('sess-e2e-1', 'worker', {
      id: 'timeline.add_clip',
      params: {
        track_id: 1,
        media_id: mediaId,
        timeline_start: 14 * sec,
        duration: 3 * sec,
        rationale: 'Insert cutaway B-roll',
        confidence: 'medium',
      },
    });
  });

  // UI should refresh from real engine queue and branch ribbon.
  await reviewCard.getByText('Branch Review: agent/cut-1').waitFor();
  await reviewCard.getByText('Cold open hook clip').waitFor();
  await reviewCard.getByText('Insert cutaway B-roll').waitFor();
  await page.locator('.ai-branch-diff-ribbon').getByText('agent/cut-1').waitFor();

  // 3. Click Stop -> halts <= 1s and late agent calls fail with SessionStopped.
  const t0 = Date.now();
  await statusChip.getByRole('button', { name: 'Stop', exact: true }).click();
  await statusChip.getByText('Stopped').waitFor();
  const elapsedMs = Date.now() - t0;
  assert.ok(elapsedMs <= 1000, `Stop should complete within <= 1s (took ${elapsedMs}ms)`);

  const stoppedErr = await page.evaluate(() => {
    try {
      window.__mockAgentExecute('sess-e2e-1', 'worker', {
        id: 'timeline.add_clip',
        params: { track_id: 1, media_id: 1, timeline_start: 20 * 254016000000, duration: 254016000000 },
      });
      return null;
    } catch (err) {
      return err instanceof Error ? err.message : String(err);
    }
  });
  assert.match(stoppedErr ?? '', /SessionStopped/, 'late agent command after Stop must be rejected');

  // Resume as human so branch workflow can proceed.
  await statusChip.getByRole('button', { name: 'Resume', exact: true }).click();
  await statusChip.getByText('Idle').waitFor();

  // 4. Reject second change (`Insert cutaway B-roll`) -> engine reverts it cleanly.
  const secondItem = reviewCard.locator('.glass', { hasText: 'Insert cutaway B-roll' });
  await secondItem.getByRole('button', { name: 'Reject (R)', exact: true }).click();
  await secondItem.getByText('Reverted').waitFor();

  // 5. Simulate human editing the first (`Cold open hook clip`) clip -> selective revert is blocked with Conflict.
  await page.evaluate(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const status = await invoke('execute_command', { request: { id: 'project.status', params: {} } });
    const v1 = status.document.project.sequence.tracks.find(t => t.id === 1);
    const clipIdx = v1.clips.findIndex(c => c.agent_note?.rationale === 'Cold open hook clip');
    await invoke('execute_command', {
      request: { id: 'timeline.trim_clip', params: { track_id: 1, clip_index: clipIdx, edge: 'end', delta: -127008000000 } },
    });
    window.__mockEmitProjectChanged?.();
  });

  const firstItem = reviewCard.locator('.glass', { hasText: 'Cold open hook clip' });
  await firstItem.getByText(/later human edit/).waitFor();
  assert.equal(await firstItem.getByRole('button', { name: 'Conflict', exact: true }).isDisabled(), true);

  // 6. Accept All switches back to `main`, merges `agent/cut-1`, and preserves actor history.
  await reviewCard.getByRole('button', { name: 'Accept All', exact: true }).click();
  await reviewCard.getByText('Branch Review: main').waitFor();

  const historySummary = await page.evaluate(() => ({
    actors: [...new Set(window.__mockHistory.map(r => r.actor))],
    hasMerge: window.__mockHistory.some(r => r.command_id === 'project.branch_merge'),
  }));
  assert.ok(historySummary.actors.includes('agent:worker'), 'history should record agent:worker');
  assert.ok(historySummary.actors.includes('human'), 'history should record human edits/merge');
  assert.equal(historySummary.hasMerge, true, 'history should record branch_merge');

  // 7. Verify explicit `?demo=1` loads demo seed without affecting `/`.
  const demoContext = await page.context().browser().newContext({ viewport: { width: 1920, height: 1080 } });
  const demoPage = await demoContext.newPage();
  try {
    await installMockBackend(demoPage, {
      mediaUrl: ctx.mockVideoUrl,
      catalogue: ctx.catalogue,
    });
    const demoUrl = new URL(page.url());
    demoUrl.search = '?demo=1';
    await demoPage.goto(demoUrl.toString());
    await demoPage.getByRole('button', { name: /Create a new project/ }).click();
    await demoPage.getByRole('button', { name: /Create project/ }).click();
    await demoPage.locator('.app-shell').waitFor();
    await demoPage.locator('.agent-status-chip').getByText('Idle').waitFor();
  } finally {
    await demoContext.close();
  }

  ctx.state.agent = 'passed';
}
