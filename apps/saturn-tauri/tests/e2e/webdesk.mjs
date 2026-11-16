// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * End-to-end verification of TS8 (Web Desk MVP):
 * 1. Entry point via `?mode=desk` mounting Web Desk browser shell.
 * 2. AGPL §13 "Source and licence" prominent link & modal notice.
 * 3. Create job from flagship recipe template (Podcast & Social Clips).
 * 4. Validation & submission transitioning job to ready/planning.
 * 5. Job Detail inspection & Plan Approval card action.
 * 6. Agent question escalation into Inbox -> structured answer -> session resumption.
 * 7. Review view: RAW proxy video surface (no glass), scrubber, timecode display,
 *    timecoded notes (add at playhead, jump to timecode, resolve with explanation),
 *    and round diff summary.
 * 8. Approve Delivery transition to delivered.
 */
import assert from 'node:assert/strict';
import { installMockBackend } from './mock-backend.mjs';

export async function run(ctx) {
  const page = ctx.page;

  // 1. Navigate to Web Desk browser entry point via URL parameter `?mode=desk`
  const baseUrl = page.url().split('?')[0];
  await page.goto(`${baseUrl}?mode=desk`);
  await installMockBackend(page, {
    mediaUrl: ctx.mockVideoUrl,
    catalogue: ctx.catalogue,
  });

  const webdeskRoot = page.locator('[data-testid="webdesk-root"]');
  await webdeskRoot.waitFor();

  // 2. Verify AGPL §13 "Source and licence" visible link and notice modal
  const agplLink = page.getByRole('link', { name: /source and licence/i });
  await agplLink.waitFor();
  assert.equal(await agplLink.isVisible(), true, 'AGPL §13 link must be visible in shell header');
  await agplLink.click();

  const agplModal = page.locator('[data-testid="agpl-license-modal"]');
  await agplModal.waitFor();
  assert.ok(await agplModal.getByText('GNU Affero General Public License').isVisible());
  assert.ok(await agplModal.getByText('AGPL §13 Remote Network Interaction Requirement:').isVisible());
  assert.ok(await agplModal.getByText('https://github.com/zvirox/easy-edits-pro').isVisible());

  // Close AGPL modal
  const understoodBtn = agplModal.getByRole('button', { name: /understood/i });
  await understoodBtn.click();
  await agplModal.waitFor({ state: 'detached' });

  // 3. Verify Jobs Board renders with metrics strip
  const metricsStrip = page.locator('[data-testid="webdesk-metrics-strip"]');
  await metricsStrip.waitFor();

  // 4. Create new job from Flagship Recipe template (Podcast & Social Clips)
  const newJobNavBtn = page.locator('[data-testid="nav-new-job"]');
  await newJobNavBtn.click();

  const newJobModal = page.locator('[data-testid="new-job-modal"]');
  await newJobModal.waitFor();

  const podcastTemplateBtn = newJobModal.locator('[data-testid="template-btn-podcast"]');
  await podcastTemplateBtn.waitFor();
  await podcastTemplateBtn.click();

  const validateBtn = newJobModal.locator('[data-testid="validate-job-btn"]');
  await validateBtn.click();

  const validationNotice = newJobModal.locator('[data-testid="validation-notice"]');
  await validationNotice.waitFor();
  const noticeText = await validationNotice.innerText();
  assert.ok(noticeText.includes('Valid sheet:'), 'Sheet validation should report valid deliverables');

  const submitBtn = newJobModal.locator('[data-testid="submit-job-btn"]');
  await submitBtn.click();

  // 5. Verify transition to Job Detail & inspect Task Sheet specs
  const detailStatusChip = page.locator('[data-testid="job-detail-status-chip"]');
  await detailStatusChip.waitFor();
  const specsCard = page.locator('[data-testid="tasksheet-specs-card"]');
  await specsCard.waitFor();
  assert.ok(await specsCard.getByText('UNTRUSTED TRANSCRIPT').isVisible(), 'Transcript input must carry untrusted tag');

  // 6. Inspect & Approve Agent Execution Plan
  const planCard = page.locator('[data-testid="plan-approval-card"]');
  await planCard.waitFor();
  const approvePlanBtn = planCard.locator('[data-testid="approve-plan-btn"]');
  await approvePlanBtn.waitFor();
  await approvePlanBtn.click();

  // Status transitions to RUNNING
  await detailStatusChip.getByText('RUNNING', { exact: true }).waitFor();

  // 7. Simulate agent question escalation into Questions Inbox
  await page.evaluate(() => {
    const question = {
      id: 'q_intro_music',
      question: 'Choose background music track for tutorial intro',
      context: 'Two license-cleared ambient loops matched pacing.',
      options: [
        { id: 'opt_ambient_tech', label: 'Ambient Tech 120bpm', is_default: true },
        { id: 'opt_acoustic_calm', label: 'Acoustic Calm 95bpm', is_default: false },
      ],
    };
    if (window.__webDeskStore?.activeJobId) {
      window.__webDeskStore.postAgentQuestion(window.__webDeskStore.activeJobId, question);
    }
    if (window.__mockAgentExecute) {
      window.__mockAgentExecute('sess-desk-e2e', 'planner', {
        id: 'tasksheet.question_ask',
        params: question,
      });
    }
  });

  // Questions Inbox appears in Job Detail
  const inboxSection = page.locator('[data-testid="questions-inbox-section"]');
  await inboxSection.waitFor();
  assert.ok(await inboxSection.getByText('Choose background music track for tutorial intro').isVisible());

  // Answer question via Inbox
  const answerBtn = inboxSection.getByRole('button', { name: /confirm choice & resume/i });
  await answerBtn.waitFor();
  await answerBtn.click();

  // 8. Open Review View
  const openReviewBtn = page.locator('[data-testid="open-review-btn"]');
  await openReviewBtn.waitFor();
  await openReviewBtn.click();

  const reviewSurface = page.locator('[data-testid="video-player-surface"]');
  await reviewSurface.waitFor();

  // Verify RAW media surface rule: must NOT carry glass classes
  const surfaceClasses = await reviewSurface.getAttribute('class');
  assert.ok(!surfaceClasses.includes('glass'), 'video player surface must be raw media, never glass');

  // Test Scrubber and Timecode display
  const timecodeDisplay = page.locator('[data-testid="timecode-display"]');
  await timecodeDisplay.waitFor();
  const initialTc = await timecodeDisplay.innerText();
  assert.ok(initialTc.length > 0, 'timecode display should render initial time');

  // Advance frame using step forward button
  const forwardFrameBtn = page.getByRole('button', { name: '+1f' });
  await forwardFrameBtn.click();

  // Add timecoded note at playhead
  const noteInput = page.locator('[data-testid="note-input"]');
  await noteInput.fill('Trim dead air in intro demonstration');
  const addNoteBtn = page.locator('[data-testid="add-note-btn"]');
  await addNoteBtn.click();

  const noteItem = page.locator('.webdesk-note-item').first();
  await noteItem.waitFor();
  assert.ok(await noteItem.getByText('Trim dead air in intro demonstration').isVisible());

  // Test timecode jump button
  const timecodeBtn = noteItem.locator('[data-testid="note-timecode-btn"]');
  await timecodeBtn.click();

  // Resolve note with explanation
  const resolveBtn = noteItem.locator('[data-testid="resolve-note-btn"]');
  await resolveBtn.click();
  const resolutionInput = noteItem.locator('[data-testid="resolution-input"]');
  await resolutionInput.fill('Trimmed 1.2s dead-air gap in take 2');
  const confirmResolveBtn = noteItem.locator('[data-testid="confirm-resolve-btn"]');
  await confirmResolveBtn.click();

  await noteItem.getByText('RESOLVED', { exact: true }).waitFor();
  assert.ok(await noteItem.getByText('Trimmed 1.2s dead-air gap in take 2').isVisible());

  // Verify Round Diff summary
  const roundDiff = page.locator('[data-testid="round-diff-summary"]');
  await roundDiff.waitFor();
  assert.ok(await roundDiff.getByText('Round Diff Summary').isVisible());

  // 9. Approve Delivery
  const approveDeliveryBtn = page.locator('[data-testid="approve-delivery-btn"]');
  await approveDeliveryBtn.click();

  // Navigate back to Jobs Board & verify DELIVERED status
  const navJobsBtn = page.locator('[data-testid="nav-jobs"]');
  await navJobsBtn.click();

  const deliveredCard = page.locator('.webdesk-job-card').first();
  await deliveredCard.waitFor();
  await deliveredCard.getByText('DELIVERED', { exact: true }).waitFor();

  ctx.state.webdesk = 'passed';
}
