// Smoke: Fairlight audio workspace - channel strips and the EQ DSP toggle.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;
  const state = ctx.state;

  const audioWorkspaceNav = page.getByRole('radiogroup', { name: 'Workspaces' });
  await audioWorkspaceNav.getByRole('radio', { name: 'Fairlight' }).click();
  await page.locator('.workspace-shell[data-workspace="audio"]').waitFor();
  await page.getByText('CHANNEL STRIP RACK').waitFor();
  await page.locator('.eq-module .dsp-power-btn').click();
  await page.waitForFunction(() => Array.from(document.querySelectorAll('.eq-module .dsp-power-btn')).some(button => button.textContent?.trim() === 'ACTIVE'));
  const audioCommand = await page.evaluate(() => window.__commandRequests.findLast(item => item.id === 'track.set_audio_processing'));
  assert.equal(audioCommand?.params?.track_id, 2, 'Fairlight EQ toggle should update the selected audio track');
  assert.equal(audioCommand?.params?.eq_enabled, true, 'Fairlight EQ toggle should send the enabled state to the editor engine');
  assert.ok(await page.locator('.fairlight-channel-strip').count() >= 2, 'Fairlight should display the project video and audio channel strips');
  state.audioCommand = audioCommand;
}
