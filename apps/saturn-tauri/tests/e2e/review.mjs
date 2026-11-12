// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
// Smoke: TS4 Review tab, round selector (v1/v2/all), round diff jump-to-tick,
// ruler note pins, N shortcut + add note at playhead, mandatory resolution
// explanation, Approve Cut gating, and low-confidence clip dot overlay.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;
  const workspaces = page.getByRole('radiogroup', { name: 'Workspaces' });
  await workspaces.getByRole('radio', { name: 'Edit' }).click();

  // Ensure a V1 clip overlaps [0, 2s] so the low-confidence AI dot renders on the clip.
  const lowConfDots = page.locator('.clip-low-confidence-dot');
  if ((await lowConfDots.count()) === 0) {
    const mediaItem = page.locator('.media-item').first();
    const mediaBounds = await mediaItem.boundingBox();
    const videoLane = page.locator('.track-lane[data-track-id="1"]');
    const laneBounds = await videoLane.boundingBox();
    await page.mouse.move(mediaBounds.x + mediaBounds.width / 2, mediaBounds.y + mediaBounds.height / 2);
    await page.mouse.down();
    await page.mouse.move(laneBounds.x + 12, laneBounds.y + laneBounds.height / 2, { steps: 10 });
    await page.mouse.up();
  }
  await lowConfDots.first().waitFor();
  assert.equal(
    await lowConfDots.first().getAttribute('aria-label'),
    'Low-confidence AI edit',
    'low-confidence AI indicator dot should expose accessible label',
  );

  // Open the "Review" tab in the right dock.
  const rightDock = page.locator('.shell-dock-right');
  const reviewTab = rightDock.getByRole('tab', { name: 'Review', exact: true });
  await reviewTab.waitFor();
  await reviewTab.click();

  const reviewCard = page.locator('.review-card');
  await reviewCard.waitFor();

  // Verify round selector includes v1 and v2, and round v2 shows Diff v1 -> v2.
  const roundSelect = reviewCard.getByLabel('Review round selector');
  await roundSelect.waitFor();
  const optionTexts = await roundSelect.locator('option').allTextContents();
  assert.ok(optionTexts.some(t => t.startsWith('v1 ')), `expected v1 option; got ${optionTexts}`);
  assert.ok(optionTexts.some(t => t.startsWith('v2 ')), `expected v2 option; got ${optionTexts}`);

  const diffRegion = reviewCard.getByRole('region', { name: 'Round diff summary' });
  await diffRegion.waitFor();
  assert.ok(
    (await diffRegion.getByText('Diff v1 → v2').count()) > 0,
    'round v2 should show diff summary from v1',
  );

  // Click diff chip and verify playhead jumps to 1.0s.
  await diffRegion.locator('.review-diff-chip').first().click();
  await page.waitForFunction(() => {
    const val = Number(document.querySelector('.ruler-lane .playhead-grip')?.getAttribute('aria-valuenow') ?? -1);
    return val > 0.5 && val < 1.5;
  });

  // Verify ruler pins reflect selected round (v2 vs all) and jump on click.
  const rulerPins = page.locator('.ruler-lane .timeline-review-pin');
  await rulerPins.first().waitFor();
  const v2PinCount = await rulerPins.count();
  await roundSelect.selectOption('all');
  await page.waitForFunction(
    prev => document.querySelectorAll('.ruler-lane .timeline-review-pin').length > prev,
    v2PinCount,
  );
  await roundSelect.selectOption('2');
  await rulerPins.first().click();
  await page.waitForFunction(() => {
    const val = Number(document.querySelector('.ruler-lane .playhead-grip')?.getAttribute('aria-valuenow') ?? -1);
    return val > 1.5 && val < 2.5;
  });

  // Approve Cut must stay disabled while open notes remain.
  const approveBtn = reviewCard.getByRole('button', { name: 'Approve Cut', exact: true });
  assert.equal(
    await approveBtn.isDisabled(),
    true,
    'Approve Cut must remain disabled while open review notes exist',
  );

  // Press N inside the Review card to focus note input, then add note at playhead.
  const noteInput = reviewCard.getByLabel('Review note text');
  await reviewCard.locator('.review-open-badge').click();
  await page.keyboard.press('n');
  const focused = await noteInput.evaluate(el => document.activeElement === el);
  assert.equal(focused, true, 'pressing N should focus the review note input');

  await noteInput.fill('Tighten reaction cut at 2s');
  await reviewCard.getByRole('button', { name: 'Add Note at Playhead', exact: true }).click();
  await reviewCard.getByText('Tighten reaction cut at 2s').waitFor();
  await reviewCard.getByText('2 open notes').waitFor();

  // Resolve all open notes (Confirm Resolve disabled until explanation is entered).
  while ((await reviewCard.getByRole('button', { name: 'Resolve / Explain', exact: true }).count()) > 0) {
    await reviewCard.getByRole('button', { name: 'Resolve / Explain', exact: true }).first().click();
    const confirmResolveBtn = reviewCard.getByRole('button', { name: 'Confirm Resolve', exact: true });
    assert.equal(
      await confirmResolveBtn.isDisabled(),
      true,
      'Confirm Resolve must stay disabled until a resolution explanation is provided',
    );
    await reviewCard.locator('.review-note-list input[type="text"]').first().fill('Trimmed cut and verified transition');
    await confirmResolveBtn.click();
  }

  await reviewCard.getByText('All notes addressed').waitFor();
  assert.equal(
    await approveBtn.isDisabled(),
    false,
    'Approve Cut should enable once all open notes are resolved with explanations',
  );
  await approveBtn.click();
  await reviewCard.getByRole('button', { name: 'Open Round', exact: true }).waitFor();

  ctx.state.review = 'passed';
}
