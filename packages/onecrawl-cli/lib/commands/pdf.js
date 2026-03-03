'use strict';

/**
 * pdf command — generate a PDF of the current page.
 *
 * Usage:
 *   onecrawl-cli pdf [file] [--format=Letter|A4|Legal] [--landscape] [--margin=<top,right,bottom,left>]
 *
 * Attempts to use the session's native pdf support first.  Falls back to
 * preparing the page for print and taking a full-page screenshot.
 *
 * @module commands/pdf
 */

const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

const VALID_FORMATS = ['Letter', 'A4', 'Legal'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'pdf',
    description: 'generate a PDF of the current page',
    usage: '[file] [--format=A4] [--landscape] [--margin=top,right,bottom,left]',
    action: pdfAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function pdfAction(args) {
  await withErrorHandling(async () => {
    const file = args._[1] || `page-${Date.now()}.pdf`;
    const format = args.format || 'A4';
    const landscape = !!args.landscape;

    if (!VALID_FORMATS.includes(format)) {
      console.error(`Invalid format "${format}". Valid: ${VALID_FORMATS.join(', ')}`);
      process.exit(1);
    }

    const margins = parseMargins(args.margin);
    const filePath = path.resolve(file);

    // Try native pdf command first
    try {
      await runSessionCommand({
        _: ['pdf', filePath],
        session: args.session,
      });
      console.log(JSON.stringify({
        saved: filePath,
        format,
        landscape,
        margins,
      }));
      return;
    } catch {
      // Native pdf not available — fall back to print-optimised screenshot
    }

    // Prepare page for print rendering
    const prepareJs = `(() => {
      const style = document.createElement('style');
      style.textContent = [
        '@media print { body { -webkit-print-color-adjust: exact; } }',
        '@page { size: ${format}${landscape ? ' landscape' : ''}; margin: ${margins.top} ${margins.right} ${margins.bottom} ${margins.left}; }',
      ].join('\\n');
      document.head.appendChild(style);
      return JSON.stringify({ prepared: true });
    })()`;

    await runSessionCommand({
      _: ['evaluate', prepareJs],
      session: args.session,
    });

    // Fall back to a full-page screenshot
    const screenshotPath = filePath.replace(/\.pdf$/i, '.png');
    await runSessionCommand({
      _: ['screenshot', '--full-page', screenshotPath],
      session: args.session,
    });

    console.log(JSON.stringify({
      saved: screenshotPath,
      format,
      landscape,
      margins,
      fallback: true,
      message: 'Native PDF not available; saved full-page screenshot instead.',
    }));
  });
}

/**
 * Parse a comma-separated margin string into individual values.
 */
function parseMargins(raw) {
  if (!raw) return { top: '0px', right: '0px', bottom: '0px', left: '0px' };
  const parts = String(raw).split(',').map(s => s.trim());
  return {
    top: parts[0] || '0px',
    right: parts[1] || parts[0] || '0px',
    bottom: parts[2] || parts[0] || '0px',
    left: parts[3] || parts[1] || parts[0] || '0px',
  };
}

module.exports = { register };
