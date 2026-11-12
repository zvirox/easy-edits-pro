// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
// Smoke: Task Sheet tab, template picker, validation, submission, Agent Inbox
// decision flow, collapsed summary row, and responsive layout across 900–1280px viewports (TS2).
import assert from 'node:assert/strict';

export async function run(ctx) {
  const page = ctx.page;

  // 1. Open the "Sheet" dock tab in the right dock.
  const rightDock = page.locator('.shell-dock-right');
  const sheetTab = rightDock.getByRole('tab', { name: 'Sheet', exact: true });
  await sheetTab.waitFor();
  await sheetTab.click();

  // Verify empty TaskSheet panel mounts without fake default sheet and exposes template selector.
  const sheetPanel = page.locator('.tasksheet-panel');
  await sheetPanel.waitFor();
  assert.equal(
    await sheetTab.getAttribute('aria-selected'),
    'true',
    'the Sheet dock tab should be active',
  );
  await sheetPanel.getByText('No Task Sheet attached').waitFor();
  const templateSelect = sheetPanel.getByLabel('Load Flagship Task Sheet Template');
  await templateSelect.selectOption('tutorial_screen');

  // Verify recipe templates are available inside loaded sheet.
  const tutorialTemplateBtn = sheetPanel.getByRole('button', {
    name: 'Tutorial & Screen Capture',
    exact: true,
  });
  await tutorialTemplateBtn.waitFor();

  // Validate the sheet.
  const validateBtn = sheetPanel.getByRole('button', { name: 'Validate', exact: true });
  await validateBtn.click();
  const validationNotice = sheetPanel.getByText('Valid sheet: 1 deliverables configured.');
  await validationNotice.waitFor();

  // Submit the sheet to transition it to "ready".
  const submitBtn = sheetPanel.getByRole('button', { name: 'Submit Sheet', exact: true });
  await submitBtn.click();

  // Verify status chip updates to READY.
  const readyChip = sheetPanel.locator('.status-chip').getByText('READY', { exact: true });
  await readyChip.waitFor();

  // Test collapsed state compact summary row within the mounted panel.
  const minimizeBtn = sheetPanel.getByRole('button', { name: 'Minimize', exact: true });
  await minimizeBtn.click();

  const collapsedRow = page.locator('.tasksheet-panel-collapsed');
  await collapsedRow.waitFor();
  assert.ok(
    (await collapsedRow.getByText('READY').count()) > 0,
    'collapsed summary row should show ready status',
  );

  // Expand back to full panel.
  const expandBtn = collapsedRow.getByRole('button', { name: 'Expand', exact: true });
  await expandBtn.click();
  await sheetPanel.waitFor();

  // Agent posts a real question into the active TaskSheet via the engine bridge.
  await page.evaluate(() => {
    window.__mockAgentExecute('sess-tasksheet-e2e', 'planner', {
      id: 'tasksheet.update',
      params: {
        status: 'needs_input',
        questions: [
          {
            id: 'q_hook_selection',
            question: 'Select hook clip for Social Reel A',
            context: 'Two candidate intro segments detected.',
            options: [
              { id: 'opt_hook_1', label: '01:24 - AI editing paradigm shift', is_default: true, description: 'High energy hook' },
              { id: 'opt_hook_2', label: '04:12 - Future of native codecs', is_default: false, description: 'Technical depth' },
            ],
            default_option_id: 'opt_hook_1',
            deadline_seconds: 300,
            status: 'open',
          },
        ],
      },
    });
  });

  // 2. Switch to the "Agent" tab to verify the active sheet header and Inbox question.
  const agentTab = rightDock.getByRole('tab', { name: 'Agent', exact: true });
  await agentTab.click();

  // Verify Agent Card displays sheet title and status chip.
  const agentCard = page.locator('.agent-card');
  await agentCard.waitFor();
  await agentCard.getByText('Software Tutorial with Screen Studio Capture').waitFor();
  await agentCard.getByText('needs input', { exact: true }).waitFor();

  // Verify Agent Inbox displays question card with options and safe default.
  const inbox = agentCard.locator('.agent-inbox');
  await inbox.waitFor();
  assert.ok(
    (await inbox.getByText('Select hook clip for Social Reel A').count()) > 0,
    'inbox should display open question',
  );
  assert.ok(
    (await inbox.getByText('Safe Default').count()) > 0,
    'inbox should indicate safe default option',
  );

  // Select second option and confirm choice to resume agent execution.
  const option2 = inbox.getByText('04:12 - Future of native codecs');
  await option2.click();

  const confirmBtn = inbox.getByRole('button', {
    name: 'Confirm Choice & Resume',
    exact: true,
  });
  await confirmBtn.click();

  // Verify answer persisted to engine and transitioned sheet status to running.
  await page.waitForTimeout(150);
  await agentCard.getByText('running', { exact: true }).waitFor();

  // 3. Test responsive layout at 1280px and 900px viewports without horizontal clipping.
  // Test 1280px viewport (in collapse band, status bar summary is visible, no overflow)
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.waitForTimeout(200);
  const overflow1280 = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  assert.equal(overflow1280, false, '1280px viewport should not produce horizontal clipping');

  const statusBarSheet = page.locator('.status-bar-sheet');
  await statusBarSheet.waitFor();
  assert.ok(
    (await statusBarSheet.getByText('running', { exact: true }).count()) > 0,
    'status bar should display active sheet status chip in 1280px viewport',
  );

  // Test 900px viewport without clipping
  await page.setViewportSize({ width: 900, height: 700 });
  await page.waitForTimeout(200);

  const overflow900 = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  assert.equal(
    overflow900,
    false,
    '900px viewport should not produce horizontal clipping',
  );
  assert.ok(
    await statusBarSheet.isVisible(),
    'status bar should display active sheet summary in 900px viewport',
  );

  // Restore standard viewport
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.waitForTimeout(200);

  ctx.state.tasksheet = 'passed';
}
