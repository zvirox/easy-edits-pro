// Smoke: timeline editing - media import, pointer drop, speed/duration,
// clip context menu, tools (razor, track select, ripple), keyboard trim,
// three-point edit, playhead commands, zoom, scrub, playback, frame steps.
import assert from 'node:assert/strict';
import { setTimeout as delay } from 'node:timers/promises';

export async function run(ctx) {
  const page = ctx.page;
  const menuBar = page.locator('.menu-bar');
  const state = ctx.state;

  await page.locator('button[data-tooltip="Import Media Files (Ctrl+I)"]').first().click();
  await page.locator('.media-item strong', { hasText: 'editor-smoke.webm' }).waitFor();

  const workspaces = page.getByRole('radiogroup', { name: 'Workspaces' });
  await workspaces.getByRole('radio', { name: 'Media' }).click();
  await page.getByText('MEDIA POOL').waitFor();
  await page.getByText('Add media to your project').waitFor();
  await page.locator('.import-media-card').waitFor();
  await page.screenshot({ path: '/tmp/easy-edits-pro-import-workspace.png', fullPage: false });
  await page.getByRole('button', { name: /editor-smoke\.webm/ }).click();
  await workspaces.getByRole('radio', { name: 'Edit' }).click();

  // Use actual pointer events to place Project media onto the V1 timeline lane.
  const mediaItem = page.locator('.media-item').first();
  const mediaBounds = await mediaItem.boundingBox();
  const videoLane = page.locator('.track-lane[data-track-id="1"]');
  const laneBounds = await videoLane.boundingBox();
  await page.mouse.move(mediaBounds.x + mediaBounds.width / 2, mediaBounds.y + mediaBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(laneBounds.x + 60, laneBounds.y + laneBounds.height / 2, { steps: 12 });
  await page.mouse.up();
  await page.getByText('Added editor-smoke.webm to timeline').waitFor();
  const clip = page.locator('.timeline-clip').first();
  // Clip > Speed/Duration changes the clip in place and undoes as one step.
  await clip.click();
  await menuBar.getByRole('button', { name: 'Clip', exact: true }).click();
  await page.locator('.menu-popover').getByText(/Speed\/Duration/).click();
  const speedDialog = page.locator('.speed-duration-dialog');
  await speedDialog.waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: '/tmp/easy-edits-pro-speed-duration.png' });
  await speedDialog.getByLabel('Speed percent').fill('200');
  await speedDialog.getByRole('button', { name: 'Apply', exact: true }).click();
  await page.waitForFunction(() =>
    window.__commandRequests.some(request => request.id === 'timeline.rate_stretch')
  );
  const speedRequest = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.rate_stretch').pop()
  );
  assert.equal(speedRequest.params.speed, 2, 'the dialog should send 2x speed');
  assert.equal(speedRequest.params.linked, false, 'an ungrouped clip has no linked audio');
  await page.locator('.clip-speed-badge').first().waitFor();
  assert.match(
    (await page.locator('.clip-speed-badge').first().textContent()) ?? '',
    /200%/,
    'the clip should show its new speed badge',
  );
  await page.locator('.app-shell').press('Control+z');
  await page.waitForFunction(() => !document.querySelector('.clip-speed-badge'));

  // The clip context menu is generated from the shipped Clip catalogue.
  await clip.click({ button: 'right' });
  const clipMenu = page.locator('.clip-context-menu');
  await clipMenu.waitFor();
  assert.equal(await clipMenu.getByRole('menuitem').count(), 5);
  const splitMenuItem = clipMenu.getByRole('menuitem', { name: /Split Clip at Playhead/ });
  assert.equal(
    await splitMenuItem.isDisabled(),
    true,
    'split should be disabled while the playhead is outside the clip',
  );
  assert.match(
    (await splitMenuItem.getAttribute('title')) ?? '',
    /outside this clip/,
    'the disabled split should explain why',
  );
  await clipMenu.getByRole('menuitem', { name: /Speed\/Duration/ }).click();
  const contextSpeedDialog = page.locator('.speed-duration-dialog');
  await contextSpeedDialog.waitFor();
  await contextSpeedDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await clip.click({ button: 'right' });
  await page.locator('.clip-context-menu').getByRole('menuitem', { name: /Remove Clip/ }).click();
  await page.waitForFunction(() => document.querySelectorAll('.timeline-clip').length === 0);
  await page.locator('.app-shell').press('Control+z');
  await page.locator('.timeline-clip').first().waitFor();

  // Every tool stamps its cursor class onto the clips it can act on.
  await page.locator('.timeline-tool-rail').getByRole('button', { name: 'Razor Tool', exact: true }).click();
  assert.ok(
    (await page.locator('.timeline-clip.tool-razor').count()) > 0,
    'the razor tool should set the razor cursor class on clips',
  );
  // Razor splits at the pointer, not just the playhead.
  const clipBox = await clip.boundingBox();
  await page.mouse.click(clipBox.x + clipBox.width * 0.5, clipBox.y + clipBox.height * 0.5);
  const razorRequests = await page.evaluate(() => window.__commandRequests);
  const razorSplit = razorRequests.filter((request) => request.id === 'timeline.split_clip').pop();
  assert.ok(
    razorSplit && typeof razorSplit.params?.at === 'number',
    'the razor tool should split at a numeric tick under the pointer',
  );
  assert.equal(
    await page.locator('.timeline-clip').count(),
    2,
    'the razor tool should split the clicked clip in two',
  );
  await page.locator('.app-shell').press('Control+z');
  await page.locator('.timeline-clip').first().waitFor();
  assert.equal(await page.locator('.timeline-clip').count(), 1, 'undo should restore the split clip');

  // Track select forward selects the clicked clip and everything after it.
  await page
    .locator('.timeline-tool-rail')
    .getByRole('button', { name: 'Track Select Forward', exact: true })
    .click();
  const restoredBox = await clip.boundingBox();
  await page.mouse.click(
    restoredBox.x + restoredBox.width * 0.5,
    restoredBox.y + restoredBox.height / 2,
  );
  await page.locator('.timeline-clip.tool-trackForward.selected').first().waitFor();
  // The Ripple tool's tail drag trims through the engine, opens the
  // Trim Monitor, and commits as a single undoable gesture.
  await page.locator('.timeline-tool-rail').getByRole('button', { name: 'Ripple Edit Tool', exact: true }).click();
  const widthBeforeTrim = await clip.evaluate(element => element.getBoundingClientRect().width);
  const trimHandle = clip.locator('.clip-trim-right');
  const handleBounds = await trimHandle.boundingBox();
  await page.mouse.move(handleBounds.x + handleBounds.width / 2, handleBounds.y + handleBounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(
    handleBounds.x + handleBounds.width / 2 - 48,
    handleBounds.y + handleBounds.height / 2,
    { steps: 8 },
  );
  await page.locator('.trim-monitor').waitFor();
  await page.mouse.up();
  await page.waitForFunction(() =>
    window.__commandRequests.some(request => request.id === 'timeline.trim_clip')
  );
  const trimRequest = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').pop()
  );
  assert.equal(trimRequest.params.edge, 'end', 'the right handle should trim the tail');
  assert.equal(trimRequest.params.ripple, true, 'the Ripple tool should request a ripple trim');
  assert.ok(trimRequest.params.delta < 0, 'dragging left should shorten the tail');
  await page.waitForFunction(
    width => document.querySelector('.timeline-clip')?.getBoundingClientRect().width < width - 1,
    widthBeforeTrim,
  );
  await page.locator('.app-shell').press('Control+z');
  await page.waitForFunction(
    width => Math.abs((document.querySelector('.timeline-clip')?.getBoundingClientRect().width ?? 0) - width) <= 1,
    widthBeforeTrim,
  );

  // Keyboard Trim Mode: enter on the selected clip, shuttle back with J, stop
  // with K, then commit one undoable trim with Enter.
  const trimsBeforeDynamic = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').length
  );
  await page.locator('.app-shell').press('Control+Alt+E');
  await page.locator('.trim-monitor-actions').waitFor();
  await page.locator('.app-shell').press('j');
  await page.waitForTimeout(250);
  await page.locator('.app-shell').press('k');
  await page.locator('.app-shell').press('Enter');
  await page.waitForFunction(
    count => window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').length > count,
    trimsBeforeDynamic,
  );
  const dynamicTrim = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.trim_clip').pop()
  );
  assert.ok(dynamicTrim.params.delta < 0, 'J should shuttle the trim backwards');
  await page.locator('.app-shell').press('Control+z');
  await page.waitForFunction(
    width => Math.abs((document.querySelector('.timeline-clip')?.getBoundingClientRect().width ?? 0) - width) <= 1,
    widthBeforeTrim,
  );

  await page.locator('.timeline-tool-rail').getByRole('button', { name: 'Selection Tool', exact: true }).click();
  // Three-point edit from the Source monitor: click the clip to focus the
  // source tab, then send its marked range to the playhead.
  await page.mouse.click(
    restoredBox.x + restoredBox.width * 0.5,
    restoredBox.y + restoredBox.height / 2,
  );
  await page.locator('.source-edit-bar').waitFor();
  await page.locator('.source-edit-bar').getByRole('button', { name: 'Insert source range', exact: true }).click();
  await page.waitForFunction(() =>
    window.__commandRequests.some(request => request.id === 'timeline.insert_clip')
  );
  const insertRequest = await page.evaluate(() =>
    window.__commandRequests.filter(request => request.id === 'timeline.insert_clip').pop()
  );
  assert.equal(typeof insertRequest.params.media_id, 'number', 'insert should carry a media id');
  assert.ok(insertRequest.params.duration > 0, 'insert should carry a positive source duration');
  await page.locator('.app-shell').press('Control+z');

  // Clicking a clip focuses the Source tab; the playhead checks below want Program.
  await page
    .locator('.monitor-unified .panel-tabs')
    .getByText('Program Timeline', { exact: true })
    .click();
  // Catalogue-bound playhead commands: End jumps to the sequence end, Home
  // returns to zero, and the transport timecode follows both.
  await page.locator('.app-shell').press('End');
  await page.waitForFunction(
    () => document.querySelector('.transport code')?.textContent !== '00:00:00'
  );
  await page.locator('.app-shell').press('Home');
  await page.waitForFunction(
    () => document.querySelector('.transport code')?.textContent === '00:00:00'
  );
  state.initialLeft = Number.parseFloat(await clip.evaluate(element => element.style.left));
  assert.ok(state.initialLeft > 30, `pointer drop should place the clip at the pointer position; got ${state.initialLeft}px`);

  const timeline = page.locator('.timeline-scroller');
  const widthBefore = await timeline.evaluate(element => element.scrollWidth);
  await page.getByRole('button', { name: 'Zoom in' }).click();
  const widthAfter = await timeline.evaluate(element => element.scrollWidth);
  assert.ok(widthAfter > widthBefore, 'zoom in should expand timeline content');
  state.zoomWidth = { before: widthBefore, after: widthAfter };

  await page.locator('.ruler-lane').click({ position: { x: 250, y: 8 } });
  const programMonitor = page.locator('.monitor').filter({ hasText: 'Program' });
  const video = programMonitor.locator('video.program-deck:not(.deck-hidden)');
  await video.waitFor();
  await page.waitForFunction(() => {
    const program = Array.from(document.querySelectorAll('.monitor')).find(panel => panel.textContent?.includes('Program'));
    const currentTime = Number(program?.querySelector('video.program-deck:not(.deck-hidden)')?.currentTime ?? 0);
    return currentTime > 1.5;
  });
  const seekTime = await video.evaluate(element => element.currentTime);
  assert.ok(seekTime > 1.5, `playhead scrub should seek Program media; got ${seekTime}s`);
  state.seekTime = seekTime;

  await page.getByRole('button', { name: 'Play clip' }).click();
  await page.getByRole('button', { name: 'Pause playback' }).waitFor();
  await delay(500);
  const playingTime = await video.evaluate(element => element.currentTime);
  assert.ok(playingTime > seekTime, `playback should advance the media clock; ${seekTime}s -> ${playingTime}s`);
  await page.getByRole('button', { name: 'Pause playback' }).click();
  await page.getByRole('button', { name: 'Play clip' }).waitFor();
  state.playingTime = playingTime;
  const timelinePlayhead = page.locator('.ruler-lane .playhead-grip');
  const pausedTime = Number(await timelinePlayhead.getAttribute('aria-valuenow'));
  await page.getByRole('button', { name: 'Next frame' }).click();
  await page.waitForFunction(minimum => {
    return Number(document.querySelector('.ruler-lane .playhead-grip')?.getAttribute('aria-valuenow') ?? 0) > minimum;
  }, pausedTime);
  const nextFrameTime = Number(await timelinePlayhead.getAttribute('aria-valuenow'));
  assert.ok(nextFrameTime > pausedTime, `next-frame should advance the preview; ${pausedTime}s -> ${nextFrameTime}s`);
  state.nextFrameTime = nextFrameTime;
  await page.getByRole('button', { name: 'Previous frame' }).click();
  await page.waitForFunction(maximum => {
    return Number(document.querySelector('.ruler-lane .playhead-grip')?.getAttribute('aria-valuenow') ?? 0) < maximum;
  }, nextFrameTime);
  const previousFrameTime = Number(await timelinePlayhead.getAttribute('aria-valuenow'));
  assert.ok(previousFrameTime < nextFrameTime, `previous-frame should move the preview backward; ${nextFrameTime}s -> ${previousFrameTime}s`);
  state.previousFrameTime = previousFrameTime;
}
