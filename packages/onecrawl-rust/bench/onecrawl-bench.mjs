import { execSync, spawn } from 'child_process';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { existsSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BIN = join(__dirname, '..', 'target', 'release', 'onecrawl');
const SESSION_FILE = '/tmp/onecrawl-session.json';

const HTML = 'data:text/html,<!DOCTYPE html><html><head><title>Benchmark</title></head><body><h1>OneCrawl Benchmark</h1><p>Testing browser performance.</p><ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul><a href="https://example.com">Link</a><button id="btn">Click Me</button><div class="card"><h2>Card</h2><p>Content here</p></div></body></html>';

const RUNS = 5;

function run(args) {
  return execSync(`${BIN} ${args}`, { timeout: 30000, stdio: ['pipe', 'pipe', 'pipe'] });
}

function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}

// Start session as detached background process, wait for session file
async function launchSession() {
  // Clean up any previous session
  try { execSync(`${BIN} session close`, { timeout: 5000, stdio: 'pipe' }); } catch {}

  const child = spawn(BIN, ['session', 'start', '--headless', '--background'], {
    stdio: 'ignore',
    detached: true,
  });
  child.unref();

  // Wait up to 30s for session file
  for (let i = 0; i < 300; i++) {
    if (existsSync(SESSION_FILE)) return;
    await sleep(100);
  }
  throw new Error('Session file never appeared');
}

async function bench() {
  const results = {};

  // Cold launch
  const t0 = performance.now();
  await launchSession();
  results.launch_ms = Math.round(performance.now() - t0);

  // Navigation
  let navTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    run(`navigate "${HTML}"`);
    navTotal += performance.now() - t;
  }
  results.nav_ms = Math.round(navTotal / RUNS);

  // Text extraction
  let textTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    run('get text');
    textTotal += performance.now() - t;
  }
  results.text_ms = Math.round(textTotal / RUNS);

  // Screenshot
  let ssTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    run('screenshot -o /tmp/onecrawl-bench-ss.png');
    ssTotal += performance.now() - t;
  }
  results.screenshot_ms = Math.round(ssTotal / RUNS);

  // JS eval
  let evalTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    run('eval "2+2"');
    evalTotal += performance.now() - t;
  }
  results.eval_ms = Math.round(evalTotal / RUNS);

  // New page/tab
  let npTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    run('new-page');
    npTotal += performance.now() - t;
  }
  results.new_page_ms = Math.round(npTotal / RUNS);

  // DOM query via eval
  let qTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    run('eval "document.querySelectorAll(\'li\').length"');
    qTotal += performance.now() - t;
  }
  results.query_ms = Math.round(qTotal / RUNS);

  // Close session
  try { run('session close'); } catch {}

  console.log(JSON.stringify({ tool: 'onecrawl', results }));
}

bench().catch(e => {
  try { execSync(`${BIN} session close`, { timeout: 10000, stdio: 'pipe' }); } catch {}
  console.error(e);
  process.exit(1);
});
