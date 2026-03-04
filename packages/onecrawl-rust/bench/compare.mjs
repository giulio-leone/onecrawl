import { execSync } from 'child_process';
import { writeFileSync, mkdirSync, existsSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const reportDir = join(__dirname, '..', 'reports', 'benchmark');
if (!existsSync(reportDir)) mkdirSync(reportDir, { recursive: true });

function runBench(label, cmd) {
  console.log(`\n--- ${label} ---`);
  try {
    const out = execSync(cmd, { cwd: __dirname, timeout: 120000, env: { ...process.env, FORCE_COLOR: '0' } });
    const json = JSON.parse(out.toString().trim().split('\n').pop());
    console.log(JSON.stringify(json.results, null, 2));
    return json;
  } catch (e) {
    console.error(`${label} failed:`, e.message);
    return { tool: label.toLowerCase(), results: {} };
  }
}

async function main() {
  console.log('=== OneCrawl vs Puppeteer vs Playwright Benchmark ===');
  console.log(`Platform: ${process.platform} ${process.arch} | Node ${process.version}\n`);

  // Build OneCrawl (release)
  console.log('Building OneCrawl (release)...');
  try {
    execSync('cargo build --release -p onecrawl-cli-rs', {
      cwd: join(__dirname, '..'),
      timeout: 300000,
      stdio: 'inherit',
      env: { ...process.env, PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}` }
    });
  } catch (e) {
    console.error('Cargo build failed, using existing binary if available');
  }

  const onecrawl = runBench('OneCrawl (Rust/CDP)', 'node onecrawl-bench.mjs');
  const puppeteer = runBench('Puppeteer', 'node puppeteer-bench.mjs');
  const playwright = runBench('Playwright', 'node playwright-bench.mjs');

  // Generate comparison report
  const metrics = ['launch_ms', 'nav_ms', 'text_ms', 'screenshot_ms', 'eval_ms', 'new_page_ms', 'query_ms'];
  const labels = {
    launch_ms:      'Browser Launch (cold)',
    nav_ms:         'Navigation (avg 5)',
    text_ms:        'Text Extraction (avg 5)',
    screenshot_ms:  'Full Screenshot (avg 5)',
    eval_ms:        'JS Evaluation (avg 5)',
    new_page_ms:    'New Page/Tab (avg 5)',
    query_ms:       'DOM Query (avg 5)',
  };

  let md = '# OneCrawl Cross-Tool Performance Comparison\n\n';
  md += `> **Date:** ${new Date().toISOString()}  \n`;
  md += `> **Platform:** ${process.platform} ${process.arch}  \n`;
  md += `> **Node.js:** ${process.version}  \n\n`;

  md += '## Results (lower is better)\n\n';
  md += '| Operation | OneCrawl (ms) | Puppeteer (ms) | Playwright (ms) | Fastest |\n';
  md += '|:----------|:-------------:|:--------------:|:---------------:|:-------:|\n';

  for (const m of metrics) {
    const oc = onecrawl.results[m] ?? '-';
    const pp = puppeteer.results[m] ?? '-';
    const pw = playwright.results[m] ?? '-';

    const vals = [
      { name: '🦀 OneCrawl', v: oc },
      { name: 'Puppeteer',   v: pp },
      { name: 'Playwright',  v: pw },
    ].filter(x => typeof x.v === 'number');

    const fastest = vals.length ? vals.sort((a, b) => a.v - b.v)[0].name : '-';

    // Bold the winner in each row
    const fmtOc = typeof oc === 'number' && fastest.includes('OneCrawl') ? `**${oc}**` : oc;
    const fmtPp = typeof pp === 'number' && fastest === 'Puppeteer'     ? `**${pp}**` : pp;
    const fmtPw = typeof pw === 'number' && fastest === 'Playwright'    ? `**${pw}**` : pw;

    md += `| ${labels[m]} | ${fmtOc} | ${fmtPp} | ${fmtPw} | ${fastest} |\n`;
  }

  // Summary: count wins
  const wins = { onecrawl: 0, puppeteer: 0, playwright: 0 };
  for (const m of metrics) {
    const entries = [
      ['onecrawl',   onecrawl.results[m]],
      ['puppeteer',  puppeteer.results[m]],
      ['playwright', playwright.results[m]],
    ].filter(([, v]) => typeof v === 'number');
    if (entries.length) {
      entries.sort((a, b) => a[1] - b[1]);
      wins[entries[0][0]]++;
    }
  }

  md += '\n## Summary\n\n';
  md += `| Tool | Wins |\n|:-----|:----:|\n`;
  md += `| 🦀 OneCrawl | ${wins.onecrawl}/${metrics.length} |\n`;
  md += `| Puppeteer | ${wins.puppeteer}/${metrics.length} |\n`;
  md += `| Playwright | ${wins.playwright}/${metrics.length} |\n`;

  md += '\n## Methodology Notes\n\n';
  md += '- **OneCrawl** uses native Rust + chromiumoxide (Chrome DevTools Protocol)\n';
  md += '- **OneCrawl CLI overhead**: each measurement includes process spawn + CDP reconnection per command. ';
  md += 'For a fair API-level comparison, use the in-process Rust benchmark (`cargo bench -p onecrawl-benchmark`)\n';
  md += '- **Puppeteer** and **Playwright** run entirely in-process (single Node.js session)\n';
  md += '- All tests use `data:text/html` URLs — zero network I/O\n';
  md += '- Metrics except Launch are averaged over 5 runs\n';
  md += '- Launch is a single cold-start measurement\n';

  const reportPath = join(reportDir, 'COMPARISON.md');
  writeFileSync(reportPath, md);
  console.log(`\n✅ Report written to: ${reportPath}`);
}

main().catch(console.error);
