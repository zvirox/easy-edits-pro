// Smoke: shell contract - welcome, catalogue-generated menus, tool rail,
// editor layout, and the sequence tab strip.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;

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
  // Graphics ships the Phase 5 caption commands and Audio ships the Phase 6
  // automation commands, so both must render; Effects ships the nodegraph
  // commands (0.1.0-alpha.13), so it must now render too.
  for (const menu of ['Audio', 'Graphics', 'Effects']) {
    await menuBar.getByRole('button', { name: menu, exact: true }).click();
    const popover = page.locator('.menu-popover').last();
    await popover.waitFor();
    const entries = await popover.locator('button').count();
    assert.ok(entries > 0, `the ${menu} menu should list shipped commands`);
    await page.keyboard.press('Escape');
    await popover.waitFor({ state: 'detached' });
  }
  // The tool rail exposes every editing and graphics tool in named groups.
  const toolRail = page.locator('.timeline-tool-rail');
  assert.equal(
    await toolRail.locator('.timeline-tool-button').count(),
    17,
    'the tool rail should expose all 17 tools',
  );
  assert.equal(
    await toolRail.locator('.timeline-tool-group').count(),
    5,
    'the tool rail should group tools into five named segments',
  );
  // Layout contract (mock skeleton): dock L | preview | dock R across the top
  // row, and a full-width NLE strip below — toolbox | timeline | meter — that
  // never moves.
  const viewport = page.viewportSize();
  const mediaBox = await page
    .locator('.shell-dock-left')
    .boundingBox();
  const inspectorDockBox = await page.locator('.shell-dock-right').boundingBox();
  const previewBox = await page.locator('.monitor-unified').boundingBox();
  const railBox = await toolRail.boundingBox();
  const timelineBox = await page.locator('.timeline').boundingBox();
  const meterBox = await page.locator('.shell-meter').boundingBox();
  assert.ok(viewport && mediaBox && inspectorDockBox && previewBox && railBox && timelineBox && meterBox);
  assert.ok(
    railBox.width <= 56,
    `the G4 toolbox should be a 52px single column (got ${railBox.width}px)`,
  );
  assert.ok(
    railBox.x <= 9 && meterBox.x + meterBox.width >= viewport.width - 9,
    'the NLE strip (toolbox + timeline + meter) should span the app width',
  );
  assert.ok(
    timelineBox.x >= railBox.x + railBox.width - 2 &&
      railBox.y >= mediaBox.y + mediaBox.height - 2,
    'the toolbox should live in the bottom row, left of the timeline',
  );
  assert.ok(
    timelineBox.x + timelineBox.width <= meterBox.x + 2 &&
      meterBox.y >= mediaBox.y + mediaBox.height - 2,
    'the master meter should close the bottom row, right of the timeline',
  );
  assert.ok(
    previewBox.x > mediaBox.x + mediaBox.width - 2 &&
      previewBox.x + previewBox.width <= inspectorDockBox.x + 2,
    'the preview should sit between the two dock cards',
  );
  assert.ok(
    railBox.y >= mediaBox.y + mediaBox.height - 2,
    'the toolbox should live in the full-width bottom row',
  );
  assert.ok(
    mediaBox.height < viewport.height * 0.75,
    'the library dock should stop above the full-width timeline',
  );

  // Sequences are created, switched, duplicated, renamed, and deleted from the
  // timeline tab strip, with the engine keeping each sequence independent.
  page.on('dialog', async (dialog) => {
    if (dialog.type() === 'prompt') await dialog.accept(ctx.dialogPrompt);
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
  ctx.dialogPrompt = 'Main edit';
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
}
