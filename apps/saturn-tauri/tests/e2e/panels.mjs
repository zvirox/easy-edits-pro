// Smoke: panels and preferences - command palette, shortcuts rebinding,
// preferences persistence, view commands, workspace switching, dock cards,
// and playhead-context for shortcut-dispatched commands.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;
  const menuBar = page.locator('.menu-bar');

  // The command palette reads the same catalogue.
  await page.keyboard.press('Control+k');
  await page.locator('#command-palette-search').fill('ripple');
  const firstPaletteResult = page.locator('.command-palette-result-label').first();
  await firstPaletteResult.waitFor();
  assert.equal(
    await firstPaletteResult.textContent(),
    'Ripple Delete',
    'the palette should rank Ripple Delete first for "ripple"',
  );
  await page.keyboard.press('Escape');

  // The shortcuts editor rebinds a command and the new key dispatches it.
  await menuBar.getByRole('button', { name: 'Help', exact: true }).click();
  await page.locator('.menu-popover').getByText('Keyboard Shortcuts', { exact: true }).click();
  const shortcutsDialog = page.locator('.shortcuts-dialog');
  await shortcutsDialog.waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-shortcuts.png' });
  const undoRow = shortcutsDialog
    .locator('.shortcuts-row')
    .filter({ has: page.getByText('Undo', { exact: true }) })
    .first();
  await undoRow.getByRole('button').click();
  await page.keyboard.press('Control+Shift+U');
  await undoRow.locator('.shortcuts-key').getByText('Ctrl+Shift+U').waitFor();
  // exact: the dialog's default label list is unique for Undo, so a non-exact
  // match would resolve to multiple rows if defaults ever collide.
  await shortcutsDialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.locator('.app-shell').press('Control+Shift+U');
  const undoRequests = await page.evaluate(() => window.__commandRequests);
  assert.ok(
    undoRequests.some((request) => request.id === 'edit.undo'),
    'a rebound shortcut should dispatch its command',
  );
  await menuBar.getByRole('button', { name: 'Help', exact: true }).click();
  await page.locator('.menu-popover').getByText('Keyboard Shortcuts', { exact: true }).click();
  await page.locator('.shortcuts-dialog').getByRole('button', { name: 'Reset all to defaults' }).click();
  await page.locator('.shortcuts-dialog').getByRole('button', { name: 'Close', exact: true }).click();

  // Preferences apply theme, glass, motion, and timeline defaults immediately.
  await page.locator('.app-shell').press('Control+,');
  const preferencesDialog = page.locator('.preferences-dialog');
  await preferencesDialog.waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-preferences.png' });
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await preferencesDialog.getByRole('button', { name: 'Light', exact: true }).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  await preferencesDialog.getByRole('button', { name: 'Dark', exact: true }).click();
  await preferencesDialog.getByRole('button', { name: 'Off', exact: true }).click();
  assert.equal(await page.locator('html').getAttribute('data-glass'), 'off');
  await preferencesDialog.getByRole('button', { name: 'Full', exact: true }).click();
  await preferencesDialog.getByRole('tab', { name: 'Timeline' }).click();
  await preferencesDialog.getByRole('button', { name: 'Compact', exact: true }).click();
  if ((await page.locator('.track-row').count()) > 0) {
    assert.ok(
      (await page.locator('.track-row.track-height-compact').count()) > 0,
      'the compact track height should reach the timeline rows',
    );
  }
  await preferencesDialog.getByRole('button', { name: 'Normal', exact: true }).click();
  await preferencesDialog.getByRole('button', { name: /Snapping on/ }).click();
  await preferencesDialog.getByRole('button', { name: /Snapping off/ }).waitFor();
  await preferencesDialog.getByRole('button', { name: /Snapping off/ }).click();
  await preferencesDialog.getByRole('button', { name: 'Close' }).click();
  const storedPreferences = await page.evaluate(() =>
    JSON.parse(localStorage.getItem('easy-edits-pro.preferences.v1') ?? '{}'),
  );
  assert.equal(storedPreferences.theme, 'dark', 'theme should persist');
  assert.equal(storedPreferences.glass, 'full', 'glass mode should persist');
  assert.equal(storedPreferences.trackHeight, 'normal', 'track height should persist');
  assert.equal(storedPreferences.snap, true, 'snapping should persist');

  // UI-only commands dispatch locally instead of hitting the engine.
  await menuBar.getByRole('button', { name: 'View', exact: true }).click();
  await page.locator('.menu-popover').getByText('Theme: Light', { exact: true }).click();
  assert.equal(
    await page.evaluate(() => document.documentElement.dataset.theme),
    'light',
    'View > Theme: Light should switch the theme',
  );
  await menuBar.getByRole('button', { name: 'View', exact: true }).click();
  await page.locator('.menu-popover').getByText('Theme: Dark', { exact: true }).click();

  // Window > Workspaces switches the routed workspace.
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Color', { exact: true }).click();
  await page.locator('.workspace-shell[data-workspace="color"]').waitFor();
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Edit', { exact: false }).click();
  await page.locator('.app-shell').waitFor();

  // G2 purged the floating-panel engine: the Edit page renders stationary
  // glass dock cards, no floats or rail, and the Window menu carries no
  // layout presets. Panel gestures now focus dock tabs.
  assert.equal(
    await page.locator('.shell-dock-left').count(),
    1,
    'the Edit page keeps a library dock',
  );
  assert.equal(
    await page.locator('.shell-dock-right').count(),
    1,
    'the Edit page keeps an inspector dock',
  );
  assert.equal(await page.locator('.floating-panel').count(), 0, 'no floating panels remain');
  assert.equal(await page.locator('.panel-rail').count(), 0, 'no panel rail remains');
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  assert.equal(
    await page.locator('.menu-popover').getByText('Layout Preset', { exact: false }).count(),
    0,
    'layout presets should be gone from the Window menu',
  );
  await page.keyboard.press('Escape');

  // The markers panel gesture focuses the Markers tab in the inspector dock.
  await page.keyboard.press('ControlOrMeta+k');
  await page.locator('.command-palette').waitFor();
  await page.locator('#command-palette-search').fill('markers panel');
  await page.locator('.command-palette-result', { hasText: 'Markers Panel' }).first().click();
  const markersTab = page
    .locator('.shell-dock-right')
    .getByRole('tab', { name: 'Markers', exact: true });
  await markersTab.waitFor();
  assert.equal(
    await markersTab.getAttribute('aria-selected'),
    'true',
    'the markers gesture should focus the dock tab',
  );
  await page.locator('.markers-panel').waitFor();

  // Shortcut-dispatched commands receive playhead context.
  await page.locator('.app-shell').press('i');
  const markRequests = await page.evaluate(() => window.__commandRequests);
  assert.ok(
    markRequests.some(
      (request) => request.id === 'timeline.set_mark_in' && typeof request.params?.at === 'number',
    ),
    'I should set the in point with the playhead tick position',
  );

  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Edit').waitFor();
  // Let the menu fade-in finish so the artifact matches the settled UI.
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-menu.png', fullPage: false });
  await page.keyboard.press('Escape');
}
