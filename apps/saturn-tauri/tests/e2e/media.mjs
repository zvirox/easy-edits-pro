// Smoke: media organisation - pointer move of a clip, then bins and asset
// metadata. Bins used to call command ids that did not exist, so this
// asserts the whole create -> rename -> assign -> delete path reaches the
// engine.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;
  const state = ctx.state;

  const clip = page.locator('.timeline-clip').first();
  const clipBounds = await clip.boundingBox();
  await page.mouse.move(clipBounds.x + 50, clipBounds.y + 18);
  await page.mouse.down();
  await page.mouse.move(clipBounds.x + 170, clipBounds.y + 18, { steps: 8 });
  await page.mouse.up();
  const movedLeft = Number.parseFloat(await page.locator('.timeline-clip').first().evaluate(element => element.style.left));
  assert.ok(movedLeft > state.initialLeft + 80, `clip pointer move should update its timeline start; ${state.initialLeft}px -> ${movedLeft}px`);
  state.movedLeft = movedLeft;

  await page.locator('[data-tooltip="Create new media bin / folder"]').click();
  const binModal = page.locator('.modal-dialog-card').filter({ hasText: 'Create New Media Bin' });
  await binModal.locator('input[type="text"]').fill('Selects');
  await binModal.getByRole('button', { name: 'Create Bin' }).click();
  await page.locator('.media-bin-pill', { hasText: 'Selects' }).waitFor();

  ctx.dialogPrompt = 'Selects 4K';
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
  state.mediaOrganisation = mediaOrganisation;

  await page.locator('.bin-close-btn').first().click();
  await page.waitForFunction(() =>
    !document.querySelector('.media-bins-strip')?.textContent?.includes('Selects 4K'),
  );
  assert.equal(
    await page.evaluate(() => window.__commandRequests.some(request => request.id === 'media.delete_bin')),
    true,
    'deleting a bin must reach media.delete_bin',
  );
}
