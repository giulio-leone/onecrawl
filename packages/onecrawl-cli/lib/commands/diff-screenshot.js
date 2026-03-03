'use strict';

/**
 * diff-screenshot command — compare screenshots against a baseline.
 *
 * Usage:
 *   onecrawl-cli diff-screenshot [--baseline=<file>] [--threshold=0.1] [--output=<file>]
 *
 * Takes a screenshot of the current page and compares it with a stored
 * baseline. Reports dimensional and file-size differences. On first run
 * saves the current screenshot as the new baseline.
 *
 * @module commands/diff-screenshot
 */

const fs = require('node:fs');
const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

const DIFF_DIR = '.onecrawl-diff';
const DEFAULT_BASELINE = path.join(DIFF_DIR, 'screenshot-baseline.png');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'diff-screenshot',
    description: 'compare page screenshots against a baseline',
    usage: '[--baseline=<file>] [--threshold=0.1] [--output=<file>]',
    action: diffScreenshotAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function diffScreenshotAction(args) {
  await withErrorHandling(async () => {
    const baselinePath = args.baseline || DEFAULT_BASELINE;
    const threshold = parseFloat(args.threshold) || 0.1;
    const outputPath = args.output || null;

    if (!fs.existsSync(DIFF_DIR)) {
      fs.mkdirSync(DIFF_DIR, { recursive: true });
    }

    // Take current screenshot
    const currentPath = path.join(DIFF_DIR, `screenshot-current-${Date.now()}.png`);
    await runSessionCommand({
      _: ['screenshot', currentPath],
      session: args.session,
    });

    if (!fs.existsSync(baselinePath)) {
      fs.copyFileSync(currentPath, baselinePath);
      fs.unlinkSync(currentPath);
      console.log(JSON.stringify({
        saved: baselinePath,
        message: 'Baseline saved. Run again to compare.',
      }));
      return;
    }

    // Compare dimensions and file sizes via canvas in the browser
    const baseB64 = fs.readFileSync(baselinePath).toString('base64');
    const currB64 = fs.readFileSync(currentPath).toString('base64');

    const js = `(async () => {
      function loadImg(b64) {
        return new Promise((resolve, reject) => {
          const img = new Image();
          img.onload = () => resolve(img);
          img.onerror = reject;
          img.src = 'data:image/png;base64,' + b64;
        });
      }
      const baseline = await loadImg(${JSON.stringify(baseB64)});
      const current = await loadImg(${JSON.stringify(currB64)});

      const w = Math.max(baseline.width, current.width);
      const h = Math.max(baseline.height, current.height);
      const canvas = document.createElement('canvas');
      canvas.width = w; canvas.height = h;
      const ctx = canvas.getContext('2d');

      // Draw baseline, sample pixels
      ctx.clearRect(0, 0, w, h);
      ctx.drawImage(baseline, 0, 0);
      const bData = ctx.getImageData(0, 0, w, h).data;

      // Draw current, sample pixels
      ctx.clearRect(0, 0, w, h);
      ctx.drawImage(current, 0, 0);
      const cData = ctx.getImageData(0, 0, w, h).data;

      let diffPixels = 0;
      const total = w * h;
      for (let i = 0; i < bData.length; i += 4) {
        const dr = Math.abs(bData[i] - cData[i]);
        const dg = Math.abs(bData[i+1] - cData[i+1]);
        const db = Math.abs(bData[i+2] - cData[i+2]);
        if (dr + dg + db > 30) diffPixels++;
      }

      return JSON.stringify({
        baselineDimensions: { width: baseline.width, height: baseline.height },
        currentDimensions: { width: current.width, height: current.height },
        totalPixels: total,
        diffPixels: diffPixels,
        mismatch: total > 0 ? +(diffPixels / total).toFixed(6) : 0,
      });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    const diff = JSON.parse(result.text);
    const pass = diff.mismatch <= threshold;

    const output = {
      mismatch: diff.mismatch,
      threshold,
      pass,
      baseline: path.resolve(baselinePath),
      current: path.resolve(currentPath),
      baselineDimensions: diff.baselineDimensions,
      currentDimensions: diff.currentDimensions,
    };

    if (outputPath) {
      fs.writeFileSync(outputPath, JSON.stringify(output, null, 2));
    }

    // Clean up temp file if passed
    if (pass) {
      fs.unlinkSync(currentPath);
    }

    console.log(JSON.stringify(output));

    if (!pass) {
      process.exit(1);
    }
  });
}

module.exports = { register };
