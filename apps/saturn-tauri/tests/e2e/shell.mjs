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
  // automation commands, so both must render; Effects stays unshipped for now.
  for (const menu of ['Audio', 'Graphics']) {
    await menuBar.getByRole('button', { name: menu, exact: true }).click();
    const popover = page.locator('.menu-popover').last();
    await popover.waitFor();
    const entries = await popover.locator('button').count();
    assert.ok(entries > 0, `the ${menu} menu should list shipped commands`);
    await page.keyboard.press('Escape');
    await popover.waitFor({ state: 'detached' });
  }
  assert.equal(
    await menuBar.getByRole('button', { name: 'Effects', exact: true }).count(),
    0,
    'the Effects menu has no shipped commands yet and must not render',
  );
  // The tool rail exposes every editing and graphics tool in named groups.
  const toolRail = page.locator('.timeline-tool-rail');
  assert.equal(
    await toolRail.locator('.timeline-tool-button').count(),
    17,
    'the tool rail should expose all 17 tools',
  );
  assert.equal(
    await toolRail.locator('.timeline-tool-group-label').count(),
    5,
    'the tool rail should group tools into five named segments',
  );
  // Layout contract: media | toolbox | preview across the top, with a
  // full-width timeline docked at the bottom edge.
  const viewport = page.viewportSize();
  const mediaBox = await page
    .locator('.floating-panel')
    .filter({ hasText: 'Media pool' })
    .boundingBox();
  const previewBox = await page.locator('.monitor-unified').boundingBox();
  const railBox = await toolRail.boundingBox();
  const timelineBox = await page.locator('.timeline').boundingBox();
  assert.ok(viewport && mediaBox && previewBox && railBox && timelineBox);
  assert.ok(
    timelineBox.x <= 1 && timelineBox.x + timelineBox.width >= viewport.width - 1,
    'the timeline should span the app width',
  );
  assert.ok(
    railBox.x >= mediaBox.x + mediaBox.width - 2 &&
      railBox.x + railBox.width <= previewBox.x + 2,
    'the toolbox should sit between the media pool and the preview',
  );
  assert.ok(
    mediaBox.height < viewport.height * 0.75,
    'the media pool should stop above the full-width timeline',
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
