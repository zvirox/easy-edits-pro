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

// G6: the status bar owns the very bottom of the app chrome and must never be
// pushed out of the viewport or covered by the workspace above it, in any
// shell mode (plan v4 G6 item 3).
async function assertStatusBarVisible(page, label) {
  const status = await page.locator('.status-bar').boundingBox();
  const shell = await page.locator('.workspace-shell').boundingBox();
  assert.ok(status, `${label}: status bar must be mounted`);
  const viewport = page.viewportSize();
  assert.ok(
    status.y + status.height <= viewport.height + TOLERANCE_PX,
    `${label}: status bar bottom (${status.y + status.height}) runs past the viewport (${viewport.height})`,
  );
  assert.ok(status.y >= 0 && status.height >= 24, `${label}: status bar has zero or negative height`);
  if (shell) {
    assert.ok(
      shell.y + shell.height <= status.y + TOLERANCE_PX,
      `${label}: workspace shell bottom (${shell.y + shell.height}) overlaps the status bar (top ${status.y})`,
    );
  }
}

// G6: at 1240px the tier-1 app bar must not let the menu strip paint over the
// right-side actions (design QA §7 "Help overlapped by icons"); at 1100px the
// menu strip scrolls instead of overflowing.
async function assertMenuBarFits(page, label) {
  const menu = await page.locator('.menu-bar').boundingBox();
  const actions = await page.locator('.system-bar-right').boundingBox();
  assert.ok(menu && actions, `${label}: app bar chrome must be mounted`);
  assert.ok(
    menu.x + menu.width <= actions.x + TOLERANCE_PX,
    `${label}: menu strip right edge (${menu.x + menu.width}) overlaps right-side actions (start ${actions.x})`,
  );
  assert.ok(menu.x + menu.width <= page.viewportSize().width, `${label}: menu strip runs past the viewport`);
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

  // G5: all eight modes route through the same shell — walk the rest too.
  // Menu labels from the command catalogue: audio is "Fairlight", keyframes is "Animation".
  const rest = ['Media', 'Fusion', 'Fairlight', 'Animation', 'Deliver'];
  for (const name of rest) {
    await switchWorkspace(page, 'Window', `Workspace: ${name}`);
    const mode = { Media: 'media', Fusion: 'fusion', Fairlight: 'audio', Animation: 'keyframes', Deliver: 'deliver' }[name];
    await page.locator(`.workspace-shell[data-workspace="${mode}"]`).waitFor();
    assertSameGeometry(edit, await snapshot(page), `edit -> ${mode}`);
    await assertStatusBarVisible(page, `${mode} status bar`);
  }

  await switchWorkspace(page, 'Window', 'Workspace: Edit');
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  assertSameGeometry(edit, await snapshot(page), 'deliver -> edit');

  ctx.state.geometryInvariance = 'passed';

  // G6 item 3: the status bar stays unobscured in every mode (checked above
  // per-mode via the shared walk; assert explicitly here too).
  await assertStatusBarVisible(page, 'edit status bar');

  // G6 item 1: menu bar must not overflow the app chrome below 1300px.
  await page.setViewportSize({ width: 1240, height: 900 });
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  await assertMenuBarFits(page, '1240px menu bar');
  await page.setViewportSize({ width: 1100, height: 900 });
  await assertMenuBarFits(page, '1100px menu bar');
  await page.setViewportSize({ width: 1920, height: 1080 });
}
