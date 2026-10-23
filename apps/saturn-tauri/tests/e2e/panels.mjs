// Smoke: panels and preferences - command palette, shortcuts rebinding,
// preferences persistence, view commands, workspace switching, layout
// presets, and playhead-context for shortcut-dispatched commands.
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
  // exact: the dialog also renders "Change shortcut for Reopen Closed Panels",
  // and a non-exact match would resolve to two buttons.
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
  await page.locator('.color-workspace-page').waitFor();
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page.locator('.menu-popover').getByText('Workspace: Edit', { exact: false }).click();
  await page.locator('.app-shell').waitFor();

  // Layout presets retract floating panels and restore them.
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page
    .locator('.menu-popover')
    .getByRole('button', { name: 'Layout Preset: Review', exact: true })
    .click();
  await page.waitForTimeout(350);
  assert.equal(
    await page.locator('.floating-panel').count(),
    0,
    'the Review preset should hide the media and inspector panels',
  );
  await page.locator('.panel-rail').waitFor();
  await menuBar.getByRole('button', { name: 'Window', exact: true }).click();
  await page
    .locator('.menu-popover')
    .getByRole('button', { name: 'Layout Preset: Learning', exact: true })
    .click();
  await page.waitForTimeout(350);
  assert.equal(
    await page.locator('.floating-panel').count(),
    2,
    'the Learning preset should restore both panels',
  );

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
