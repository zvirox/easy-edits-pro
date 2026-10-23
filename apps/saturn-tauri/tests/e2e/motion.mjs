// Smoke: reduced motion is a hard contract. Emulating
// `prefers-reduced-motion: reduce` must match the media query and collapse
// every CSS animation to an instant frame, while mount/unmount surfaces
// (command palette) still work.
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;
  const state = ctx.state;

  await page.emulateMedia({ reducedMotion: 'reduce' });
  const reducedMotion = await page.evaluate(() => {
    const probe = document.createElement('div');
    probe.className = 'preview-preparing-dot';
    document.body.appendChild(probe);
    const raw = getComputedStyle(probe).animationDuration;
    probe.remove();
    const seconds = raw.endsWith('ms') ? Number.parseFloat(raw) / 1000 : Number.parseFloat(raw);
    return {
      mediaQueryMatched: window.matchMedia('(prefers-reduced-motion: reduce)').matches,
      probeAnimationSeconds: seconds,
      raw,
    };
  });
  assert.equal(reducedMotion.mediaQueryMatched, true, 'emulated reduced motion should match the media query');
  assert.ok(reducedMotion.probeAnimationSeconds <= 0.05, `reduced motion should collapse CSS animations; ${reducedMotion.raw}`);
  await page.keyboard.press('ControlOrMeta+k');
  await page.locator('.command-palette').waitFor();
  await page.keyboard.press('Escape');
  await page.locator('.command-palette').waitFor({ state: 'detached' });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  state.reducedMotion = reducedMotion;
}
