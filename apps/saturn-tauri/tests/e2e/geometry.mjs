// G3: Geometry Invariance (plan v4 directive 5). The persistent NLE skeleton
// — program monitor, toolbox, timeline — is mounted once by WorkspaceShell and
// must never move or remount when the workspace mode changes. Only the dock
// cards re-tab per mode.
import assert from 'node:assert/strict';

const TOLERANCE_PX = 1;

// Probe stamping: a random id is assigned to an element the first time it is
// seen. If the element ever remounts, the fresh element has no probe and gets
// a new one — so an unchanged probe proves continuous mount.
async function probe(page, selector) {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    el.dataset.geometryProbe ||= Math.random().toString(36).slice(2);
    return el.dataset.geometryProbe;
  }, selector);
}

async function snapshot(page) {
  const monitor = await page.locator('.monitor-unified').boundingBox();
  const rail = await page.locator('.timeline-tool-rail').boundingBox();
  const timeline = await page.locator('.timeline').boundingBox();
  assert.ok(monitor && rail && timeline, 'the persistent skeleton must be mounted in every shell mode');
  const probes = {
    monitor: await probe(page, '.monitor-unified'),
    rail: await probe(page, '.timeline-tool-rail'),
    timeline: await probe(page, '.timeline'),
  };
  assert.ok(probes.monitor && probes.rail && probes.timeline, 'probes must stamp real elements');
  return { monitor, rail, timeline, probes };
}

function assertSameGeometry(before, after, label) {
  for (const key of ['monitor', 'rail', 'timeline']) {
    const a = before[key];
    const b = after[key];
    for (const dim of ['x', 'y', 'width', 'height']) {
      assert.ok(
        Math.abs(a[dim] - b[dim]) <= TOLERANCE_PX,
        `${label}: ${key}.${dim} moved ${Math.abs(a[dim] - b[dim]).toFixed(2)}px (${a[dim]} -> ${b[dim]})`,
      );
    }
    assert.equal(before.probes[key], after.probes[key], `${label}: ${key} remounted (probe changed)`);
  }
}

async function switchWorkspace(page, menu, name) {
  await page.locator('.menu-bar').getByRole('button', { name: menu, exact: true }).click();
  await page.locator('.menu-popover').getByText(name, { exact: true }).click();
  await page.locator('.menu-popover').waitFor({ state: 'detached' });
}

export async function run(ctx) {
  const page = ctx.page;

  // panels.mjs leaves the app in Edit mode; verify the shell before switching.
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  const edit = await snapshot(page);

  await switchWorkspace(page, 'Window', 'Workspace: Color');
  await page.locator('.workspace-shell[data-workspace="color"]').waitFor();
  assertSameGeometry(edit, await snapshot(page), 'edit -> color');

  await switchWorkspace(page, 'Window', 'Workspace: Effects');
  await page.locator('.workspace-shell[data-workspace="effects"]').waitFor();
  assertSameGeometry(edit, await snapshot(page), 'color -> effects');

  await switchWorkspace(page, 'Window', 'Workspace: Edit');
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  assertSameGeometry(edit, await snapshot(page), 'effects -> edit');

  ctx.state.geometryInvariance = 'passed';
}
