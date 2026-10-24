// G3: Geometry Invariance (plan v4 directive 5). The persistent NLE skeleton
// — program monitor, toolbox, timeline — is mounted once by WorkspaceShell and
// must never move or remount when the workspace mode changes. Only the dock
// cards re-tab per mode.
//
// §9 acceptance (plan v4): the walk runs at 1920x1080, 1600x900 AND 1280x720;
// additionally no two card surfaces may intersect in any mode ("no-overlap
// test"), and the status bar must stay visible everywhere.
// §5 column presets (plan v4 changes note 3): a workspace may pick a different
// dock/monitor column ratio from the small fixed table while "timeline,
// toolbox, meters and bars stay identical". The invariance contract is
// therefore: row 2 (toolbox + timeline) never moves; the monitor keeps its row
// position and its right edge (the right dock is locked at 24%); modes sharing
// the edit preset keep the monitor box pixel-identical; modes with a different
// preset must visibly re-ratio (the shift must actually happen).
// Exception (user decision, 2026-10-23): Deliver is the previous full-page
// ExportWorkspace — no persistent skeleton there, so it is asserted separately.
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
// menu strip scrolls instead of overflowing. The bboxes only prove the boxes
// fit, so also assert every menu trigger's right edge stays left of the file
// actions — child overflow inside a shrunken .menu-bar must fail here too.
async function assertMenuBarFits(page, label) {
  const menu = await page.locator('.menu-bar').boundingBox();
  const actions = await page.locator('.system-bar-right').boundingBox();
  assert.ok(menu && actions, `${label}: app bar chrome must be mounted`);
  assert.ok(
    menu.x + menu.width <= actions.x + TOLERANCE_PX,
    `${label}: menu strip right edge (${menu.x + menu.width}) overlaps right-side actions (start ${actions.x})`,
  );
  assert.ok(menu.x + menu.width <= page.viewportSize().width, `${label}: menu strip runs past the viewport`);
  const edges = await page.evaluate(() => {
    const strip = document.querySelector('.menu-bar');
    const fileActions = document.querySelector('.file-actions');
    if (!strip || !fileActions) return null;
    const clip = strip.getBoundingClientRect();
    // .menu-bar scrolls (overflow-x: auto), so a trigger's layout rect can
    // extend past the strip's visible box; only the clipped intersection is
    // painted or clickable.
    const visibleRights = [...document.querySelectorAll('.menu-bar .app-menu-trigger')].map((t) => {
      const r = t.getBoundingClientRect();
      return Math.min(r.right, clip.right);
    });
    return { lastRight: Math.max(0, ...visibleRights), actionsLeft: fileActions.getBoundingClientRect().x };
  });
  assert.ok(edges, `${label}: file actions must be mounted`);
  assert.ok(
    edges.lastRight <= edges.actionsLeft + TOLERANCE_PX,
    `${label}: a menu trigger (right edge ${edges.lastRight.toFixed(1)}) paints over the file actions (start ${edges.actionsLeft.toFixed(1)})`,
  );
}

// §9 no-overlap test: no two card surfaces may intersect (menus and modals
// aside — menus are closed during this walk). Card surfaces are the visible
// glass/panel blocks inside the shell plus the status bar; the 8px grid gap
// means any pixel intersection on both axes is a real overlap.
const CARD_SELECTORS = [
  '.shell-dock-left',
  '.shell-dock-right',
  '.monitor-unified',
  '.shell-toolbox',
  '.shell-timeline',
  '.shell-meter',
  '.status-bar',
];

async function assertNoCardOverlap(page, label) {
  // Dock cards mount with a 220ms fade+rise (G6); let the settle finish so a
  // mid-animation transform cannot fake an overlap.
  await page.waitForTimeout(280);
  const boxes = await page.evaluate((selectors) => {
    return selectors
      .flatMap((sel) => [...document.querySelectorAll(sel)])
      .map((el) => {
        const r = el.getBoundingClientRect();
        return { name: `${el.tagName.toLowerCase()}.${[...el.classList].slice(0, 2).join('.')}`, x: r.x, y: r.y, w: r.width, h: r.height };
      })
      .filter((b) => b.w > 0 && b.h > 0);
  }, CARD_SELECTORS);
  for (let i = 0; i < boxes.length; i++) {
    for (let j = i + 1; j < boxes.length; j++) {
      const a = boxes[i];
      const b = boxes[j];
      const overlapX = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
      const overlapY = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
      assert.ok(
        overlapX <= TOLERANCE_PX || overlapY <= TOLERANCE_PX,
        `${label}: cards overlap — ${a.name} and ${b.name} intersect by ` +
          `${Math.max(overlapX, overlapY).toFixed(1)}px`,
      );
    }
  }
  const viewport = page.viewportSize();
  for (const b of boxes) {
    assert.ok(
      b.x >= -TOLERANCE_PX && b.y >= -TOLERANCE_PX &&
        b.x + b.w <= viewport.width + TOLERANCE_PX &&
        b.y + b.h <= viewport.height + TOLERANCE_PX,
      `${label}: card ${b.name} escapes the viewport (${viewport.width}x${viewport.height})`,
    );
  }
}

// G7 debt (plan v4 §5): in the 1100–1280 band the right dock card collapses
// so the persistent monitor/toolbox/timeline keep their room. The card must
// actually be out of layout (display:none), and the shell-top grid must drop
// to two tracks instead of squeezing three columns.
async function assertRightDockCollapsed(page, label) {
  const box = await page.locator('.shell-dock-right').boundingBox();
  const collapsed = box === null || box.width === 0 || box.height === 0;
  assert.ok(collapsed, `${label}: right dock card must collapse in the 1100–1280 band`);
  const tracks = await page.evaluate(() =>
    getComputedStyle(document.querySelector('.shell-top')).gridTemplateColumns.split(' ').length,
  );
  assert.equal(tracks, 2, `${label}: shell-top must fall back to two grid tracks`);
}

// Mirror of ui/src/workspaces/columnRatios.ts — the plan §5 fixed table the
// harness asserts against (kept literal here so the e2e test pins the plan's
// numbers, not just the implementation's current values).
const PRESETS = {
  edit: [28, 48, 24],
  media: [28, 48, 24],
  effects: [28, 48, 24],
  fusion: [28, 48, 24],
  keyframes: [28, 48, 24],
  color: [34, 42, 24],
  audio: [40, 36, 24],
};

function assertModeGeometry(baseline, after, mode, label, { expectRemount = false, collapseBand = false } = {}) {
  const [leftRatio] = PRESETS[mode] ?? PRESETS.edit;
  const sharesEditPreset = leftRatio === PRESETS.edit[0];

  // Row 2 (toolbox + timeline) never moves in any mode, and never remounts.
  for (const key of ['rail', 'timeline']) {
    const a = baseline[key];
    const b = after[key];
    for (const dim of ['x', 'y', 'width', 'height']) {
      assert.ok(
        Math.abs(a[dim] - b[dim]) <= TOLERANCE_PX,
        `${label}: ${key}.${dim} moved ${Math.abs(a[dim] - b[dim]).toFixed(2)}px (${a[dim]} -> ${b[dim]})`,
      );
    }
    if (expectRemount) continue;
    assert.equal(baseline.probes[key], after.probes[key], `${label}: ${key} remounted (probe changed)`);
  }

  // Row 1: the monitor keeps its row slot; its column may re-ratio per the
  // §5 preset table — but only in the full three-track shell (above 1280px).
  // Inside the 1100–1280 collapse band the shell is a fixed two-track
  // responsive regime, so the monitor is invariant again there. The right
  // edge stays fixed either way (right dock locked at 24% or collapsed).
  const a = baseline.monitor;
  const b = after.monitor;
  for (const dim of ['y', 'height']) {
    assert.ok(
      Math.abs(a[dim] - b[dim]) <= TOLERANCE_PX,
      `${label}: monitor.${dim} moved ${Math.abs(a[dim] - b[dim]).toFixed(2)}px (${a[dim]} -> ${b[dim]})`,
    );
  }
  if (collapseBand) {
    for (const dim of ['x', 'width']) {
      assert.ok(
        Math.abs(a[dim] - b[dim]) <= TOLERANCE_PX,
        `${label}: monitor.${dim} moved ${Math.abs(a[dim] - b[dim]).toFixed(2)}px (${a[dim]} -> ${b[dim]}) ` +
          'inside the collapse band, a fixed two-track regime',
      );
    }
    if (expectRemount) return;
    assert.equal(baseline.probes.monitor, after.probes.monitor, `${label}: monitor remounted (probe changed)`);
    return;
  }
  assert.ok(
    Math.abs((a.x + a.width) - (b.x + b.width)) <= TOLERANCE_PX,
    `${label}: monitor right edge moved (${(a.x + a.width).toFixed(2)} -> ${(b.x + b.width).toFixed(2)}) — ` +
      'the §5 table locks the right dock at 24%',
  );
  if (sharesEditPreset) {
    for (const dim of ['x', 'width']) {
      assert.ok(
        Math.abs(a[dim] - b[dim]) <= TOLERANCE_PX,
        `${label}: monitor.${dim} moved ${Math.abs(a[dim] - b[dim]).toFixed(2)}px (${a[dim]} -> ${b[dim]})`,
      );
    }
  } else {
    assert.ok(
      Math.abs(a.x - b.x) > TOLERANCE_PX,
      `${label}: ${mode} preset (left ${leftRatio}%) did not re-ratio the monitor column`,
    );
  }
  if (expectRemount) return;
  assert.equal(baseline.probes.monitor, after.probes.monitor, `${label}: monitor remounted (probe changed)`);
}

async function switchWorkspace(page, menu, name) {
  await page.locator('.menu-bar').getByRole('button', { name: menu, exact: true }).click();
  await page.locator('.menu-popover').getByText(name, { exact: true }).click();
  await page.locator('.menu-popover').waitFor({ state: 'detached' });
}

const MODE_WALK = [
  { menu: 'Window', name: 'Workspace: Color', mode: 'color' },
  { menu: 'Window', name: 'Workspace: Effects', mode: 'effects' },
  // G5: the docked modes route through the same shell. Menu labels from the
  // command catalogue: audio is "Fairlight", keyframes is "Animation".
  // Deliver is NOT in the walk: it is a full-page workspace by user decision
  // (2026-10-23) — it never carries the persistent shell, so geometry
  // invariance covers the seven shell modes only (assertDeliverPage below).
  { menu: 'Window', name: 'Workspace: Media', mode: 'media' },
  { menu: 'Window', name: 'Workspace: Fusion', mode: 'fusion' },
  { menu: 'Window', name: 'Workspace: Fairlight', mode: 'audio' },
  { menu: 'Window', name: 'Workspace: Animation', mode: 'keyframes' },
  { menu: 'Window', name: 'Workspace: Edit', mode: 'edit' },
];

// §9 acceptance: "read the bounding boxes of the top bar, toolbox, timeline,
// meters and status bar. Assert they are identical across workspaces (within
// 1px)" at every acceptance viewport. Baseline is edit mode at that viewport.
async function sweepViewport(page, ctx, width, height) {
  const label = `${width}x${height}`;
  await page.setViewportSize({ width, height });
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  const baseline = await snapshot(page);
  for (const step of MODE_WALK) {
    if (step.mode === 'edit') continue; // baseline mode
    await switchWorkspace(page, step.menu, step.name);
    await page.locator(`.workspace-shell[data-workspace="${step.mode}"]`).waitFor();
    assertModeGeometry(baseline, await snapshot(page), step.mode, `${label}: edit -> ${step.mode}`, {
      collapseBand: width <= 1280,
    });
    await assertStatusBarVisible(page, `${label} ${step.mode} status bar`);
    await assertMenuBarFits(page, `${label} ${step.mode} menu bar`);
    await assertNoCardOverlap(page, `${label} ${step.mode}`);
  }
  // Deliver: full-page exception — the page mounts with no persistent
  // skeleton, but the status bar stays visible and the menu bar stays sane.
  // (assertStatusBarVisible would hang on its .workspace-shell lookup here,
  // so the bare status-bar checks are inlined.)
  await switchWorkspace(page, 'Window', 'Workspace: Deliver');
  await page.locator('.export-workspace').waitFor();
  const deliverStatus = await page.locator('.status-bar').boundingBox();
  assert.ok(deliverStatus, `${label} deliver: status bar must be mounted`);
  const deliverViewport = page.viewportSize();
  assert.ok(
    deliverStatus.y >= 0 &&
      deliverStatus.y + deliverStatus.height <= deliverViewport.height + TOLERANCE_PX,
    `${label} deliver: status bar escapes the viewport`,
  );
  await assertMenuBarFits(page, `${label} deliver menu bar`);

  await switchWorkspace(page, 'Window', 'Workspace: Edit');
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  // Deliver unmounts the shell by design (full-page workspace, user decision
  // 2026-10-23), so returning to Edit remounts the skeleton once — box
  // invariance still holds, but the probe continuity check does not apply.
  assertModeGeometry(baseline, await snapshot(page), 'edit', `${label}: deliver -> edit`, { expectRemount: true });
  await assertNoCardOverlap(page, `${label} edit`);
  await assertMenuBarFits(page, `${label} edit menu bar`);
}

export async function run(ctx) {
  const page = ctx.page;

  // panels.mjs leaves the app in Edit mode; verify the shell before the sweep.
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  await sweepViewport(page, ctx, 1920, 1080);
  await sweepViewport(page, ctx, 1600, 900);
  await sweepViewport(page, ctx, 1280, 720);

  ctx.state.geometryInvariance = 'passed (1920/1600/1280, seven shell modes + full-page deliver)';
  ctx.state.noCardOverlap = 'passed (all modes, three viewports)';

  // G7 debt (plan v4 §5): the right-dock collapse band. Above 1280 the card
  // mounts; inside 1100–1280 it must be out of layout while every shell
  // invariant (persistent zones mounted, status bar visible, menu strip sane,
  // no card overlap) still holds.
  await page.setViewportSize({ width: 1281, height: 900 });
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  await page.waitForTimeout(320);
  assert.ok(
    await page.locator('.shell-dock-right').isVisible(),
    '1281px: right dock must be mounted above the collapse band',
  );
  await page.setViewportSize({ width: 1150, height: 900 });
  await page.waitForTimeout(320);
  await snapshot(page);
  await assertRightDockCollapsed(page, '1150px collapse band');
  await assertStatusBarVisible(page, '1150px status bar');
  await assertMenuBarFits(page, '1150px menu bar');
  await assertNoCardOverlap(page, '1150px collapse band');
  ctx.state.rightDockCollapse = 'passed (collapses across 1100–1280, mounts at 1281px, invariants hold)';

  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();

  // G6 item 1: menu bar must not overflow the app chrome below 1300px.
  await page.setViewportSize({ width: 1240, height: 900 });
  await page.locator('.workspace-shell[data-workspace="edit"]').waitFor();
  await assertMenuBarFits(page, '1240px menu bar');
  await page.setViewportSize({ width: 1100, height: 900 });
  await assertMenuBarFits(page, '1100px menu bar');
  await page.setViewportSize({ width: 1920, height: 1080 });
}
