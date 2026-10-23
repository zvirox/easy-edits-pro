// Smoke: Deliver workspace - export surface renders and the hardware
// encoder option tracks the selected format (MP4 yes, WebM no).
import assert from 'node:assert/strict';
import { setTimeout as delay } from 'node:timers/promises';

export async function run(ctx) {
  const page = ctx.page;

  const workspaceNav = page.getByRole('radiogroup', { name: 'Workspaces' });
  await workspaceNav.getByRole('radio', { name: 'Deliver' }).click();
  await page.getByText('Media File', { exact: true }).waitFor();
  const hardwareEncoding = page.getByRole('checkbox', { name: /Use hardware encoder when available/ });
  assert.ok(await hardwareEncoding.isEnabled(), 'hardware encoding should be selectable for MP4');
  await page.getByLabel('Format').selectOption('webm');
  assert.equal(await hardwareEncoding.isEnabled(), false, 'hardware H.264 encoding should be disabled for WebM');
  await page.getByLabel('Format').selectOption('mp4');
  await delay(100);
  await page.screenshot({ path: '/tmp/easy-edits-pro-export-workspace.png', fullPage: false });
  await workspaceNav.getByRole('radio', { name: 'Edit' }).click();
}
