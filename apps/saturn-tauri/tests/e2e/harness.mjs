// Shared harness for the Playwright UI smoke suite (`pnpm test:e2e`).
// It boots the Vite dev server, opens Chromium, installs the mocked media
// route, and hands every area module the same `ctx` (page + shared state).
import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { chromium } from 'playwright';

const appDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

async function waitForServer(server, serverOutput, outputAppend) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) throw new Error(`Vite exited early:\n${serverOutput}`);
    try {
      const response = await fetch('http://127.0.0.1:1420/');
      if (response.ok) return;
    } catch { /* Vite is not listening yet. */ }
    await delay(200);
  }
  throw new Error(`Timed out waiting for Vite at 127.0.0.1:1420:\n${serverOutput}`);
}

export async function createHarness() {
  const fixture = await readFile(path.join(appDirectory, 'tests/fixtures/editor-smoke.webm'));
  const catalogue = JSON.parse(
    await readFile(path.join(appDirectory, '../../docs/reference/command-catalogue.json'), 'utf8'),
  );
  const server = spawn('pnpm', ['dev', '--host', '127.0.0.1', '--strictPort'], {
    cwd: appDirectory,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let serverOutput = '';
  server.stdout.on('data', chunk => { serverOutput += chunk.toString(); });
  server.stderr.on('data', chunk => { serverOutput += chunk.toString(); });

  const harness = {
    appDirectory,
    fixture,
    mockVideoUrl: `data:video/webm;base64,${fixture.toString('base64')}`,
    catalogue,
    server,
    // Shared mutable state flowing between the area modules and the summary.
    state: {},
    // Mutable prompt answer consumed by the window 'dialog' handler; modules
    // change it before triggering a prompt-based flow (sequences, bin rename).
    dialogPrompt: 'B roll',
  };

  harness.waitForServer = () => waitForServer(server, serverOutput, () => {});
  harness.teardown = async () => {
    if (harness.browser) await harness.browser.close();
    server.kill('SIGTERM');
  };

  harness.browser = await chromium.launch({ headless: true });
  const page = await harness.browser.newPage({ viewport: { width: 1920, height: 1080 } });
  page.setDefaultTimeout(8_000);
  harness.page = page;
  harness.pageErrors = [];
  page.on('pageerror', error => harness.pageErrors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') harness.pageErrors.push(message.text()); });

  // Mock the media server: range requests against the synthetic fixture are
  // answered locally so the preview plays without a real backend.
  await page.route('http://127.0.0.1:1421/media**', async route => {
    const range = route.request().headers().range;
    const match = range?.match(/^bytes=(\d+)-(\d*)$/);
    const start = match ? Number(match[1]) : 0;
    const end = match?.[2] ? Math.min(Number(match[2]), fixture.length - 1) : fixture.length - 1;
    const headers = {
      'Accept-Ranges': 'bytes',
      'Access-Control-Allow-Origin': 'http://127.0.0.1:1420',
      'Content-Type': 'video/webm',
    };
    if (match) {
      headers['Content-Range'] = `bytes ${start}-${end}/${fixture.length}`;
      await route.fulfill({ status: 206, headers, body: fixture.subarray(start, end + 1) });
    } else {
      await route.fulfill({ headers, body: fixture });
    }
  });

  return harness;
}
