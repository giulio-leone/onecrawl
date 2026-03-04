import { chromium } from 'playwright';

const HTML = `data:text/html,<!DOCTYPE html><html><head><title>Benchmark</title></head><body>
<h1>OneCrawl Benchmark</h1>
<p>Testing browser performance.</p>
<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>
<a href="https://example.com">Link</a>
<button id="btn">Click Me</button>
<div class="card"><h2>Card</h2><p>Content here</p></div>
</body></html>`;

const RUNS = 5;

async function bench() {
  const results = {};

  // Cold launch
  const t0 = performance.now();
  const browser = await chromium.launch({ headless: true });
  results.launch_ms = Math.round(performance.now() - t0);

  const context = await browser.newContext();
  const page = await context.newPage();

  // Navigation
  let navTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    await page.goto(HTML, { waitUntil: 'domcontentloaded' });
    navTotal += performance.now() - t;
  }
  results.nav_ms = Math.round(navTotal / RUNS);

  // Text extraction
  let textTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    await page.evaluate(() => document.body.innerText);
    textTotal += performance.now() - t;
  }
  results.text_ms = Math.round(textTotal / RUNS);

  // Screenshot
  let ssTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    await page.screenshot({ fullPage: true });
    ssTotal += performance.now() - t;
  }
  results.screenshot_ms = Math.round(ssTotal / RUNS);

  // JS eval
  let evalTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    await page.evaluate(() => 2 + 2);
    evalTotal += performance.now() - t;
  }
  results.eval_ms = Math.round(evalTotal / RUNS);

  // New page/tab
  let npTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    const p = await context.newPage();
    npTotal += performance.now() - t;
    await p.close();
  }
  results.new_page_ms = Math.round(npTotal / RUNS);

  // DOM query
  let qTotal = 0;
  for (let i = 0; i < RUNS; i++) {
    const t = performance.now();
    await page.locator('li').count();
    qTotal += performance.now() - t;
  }
  results.query_ms = Math.round(qTotal / RUNS);

  await browser.close();
  console.log(JSON.stringify({ tool: 'playwright', results }));
}

bench().catch(e => {
  console.error(e);
  process.exit(1);
});
