// Smoke: markers - M adds at the playhead, Shift+M marks a range, the lane
// renders both, Shift+Right jumps to the next marker, and the Markers panel
// lists and deletes them without leaving the editor.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;
  const state = ctx.state;

  await page.locator('.timeline-ruler').click({ position: { x: 320, y: 12 } });
  await page.keyboard.press('m');
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
  state.markerRequests = markerRequests;
  const markerCount = await page.locator('.marker-row').count();
  await page.locator('.marker-row .marker-delete').first().click();
  await page.waitForFunction(() => document.querySelectorAll('.marker-row').length === 1);
  assert.equal(await page.locator('.marker-row').count(), 1, 'deleting from the panel removes the marker');
  state.markerCount = markerCount;
}
