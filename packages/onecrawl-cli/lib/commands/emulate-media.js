'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * emulate-media command — override CSS media features.
 *
 * Usage:
 *   onecrawl-cli emulate-media [--color-scheme=dark|light|no-preference]
 *                               [--reduced-motion=reduce|no-preference]
 *                               [--forced-colors=active|none]
 *                               [--media=screen|print]
 *
 * @module commands/emulate-media
 */

const VALID_COLOR_SCHEMES = ['light', 'dark', 'no-preference'];
const VALID_REDUCED_MOTION = ['reduce', 'no-preference'];
const VALID_FORCED_COLORS = ['active', 'none'];
const VALID_MEDIA = ['screen', 'print'];

function register(registry) {
  registry.add({
    name: 'emulate-media',
    description: 'override CSS media features (color-scheme, reduced-motion, forced-colors, media type)',
    usage: '[--color-scheme=dark|light|no-preference] [--reduced-motion=reduce|no-preference] [--forced-colors=active|none] [--media=screen|print]',
    action: emulateMediaAction,
  });
}

async function emulateMediaAction(args) {
  await withErrorHandling(async () => {
    const colorScheme = args['color-scheme'] || null;
    const reducedMotion = args['reduced-motion'] || null;
    const forcedColors = args['forced-colors'] || null;
    const media = args.media || null;

    if (!colorScheme && !reducedMotion && !forcedColors && !media) {
      console.error(
        'Usage: onecrawl-cli emulate-media [--color-scheme=dark|light|no-preference]\n' +
        '                                   [--reduced-motion=reduce|no-preference]\n' +
        '                                   [--forced-colors=active|none]\n' +
        '                                   [--media=screen|print]'
      );
      process.exit(1);
    }

    if (colorScheme && !VALID_COLOR_SCHEMES.includes(colorScheme)) {
      console.error(`Invalid --color-scheme: "${colorScheme}". Must be one of: ${VALID_COLOR_SCHEMES.join(', ')}`);
      process.exit(1);
    }
    if (reducedMotion && !VALID_REDUCED_MOTION.includes(reducedMotion)) {
      console.error(`Invalid --reduced-motion: "${reducedMotion}". Must be one of: ${VALID_REDUCED_MOTION.join(', ')}`);
      process.exit(1);
    }
    if (forcedColors && !VALID_FORCED_COLORS.includes(forcedColors)) {
      console.error(`Invalid --forced-colors: "${forcedColors}". Must be one of: ${VALID_FORCED_COLORS.join(', ')}`);
      process.exit(1);
    }
    if (media && !VALID_MEDIA.includes(media)) {
      console.error(`Invalid --media: "${media}". Must be one of: ${VALID_MEDIA.join(', ')}`);
      process.exit(1);
    }

    const emulated = {};
    if (colorScheme) emulated.colorScheme = colorScheme;
    if (reducedMotion) emulated.reducedMotion = reducedMotion;
    if (forcedColors) emulated.forcedColors = forcedColors;
    if (media) emulated.media = media;

    const js = `(() => {
      if (!window.__onecrawl_orig_matchMedia) {
        window.__onecrawl_orig_matchMedia = window.matchMedia.bind(window);
      }
      const origMM = window.__onecrawl_orig_matchMedia;
      const overrides = window.__onecrawl_media_overrides || {};
      ${colorScheme ? `overrides.colorScheme = ${JSON.stringify(colorScheme)};` : ''}
      ${reducedMotion ? `overrides.reducedMotion = ${JSON.stringify(reducedMotion)};` : ''}
      ${forcedColors ? `overrides.forcedColors = ${JSON.stringify(forcedColors)};` : ''}
      ${media ? `overrides.media = ${JSON.stringify(media)};` : ''}
      window.__onecrawl_media_overrides = overrides;

      function makeResult(matches, query) {
        return {
          matches: matches, media: query, onchange: null,
          addListener: function() {}, removeListener: function() {},
          addEventListener: function() {}, removeEventListener: function() {},
          dispatchEvent: function() { return true; },
        };
      }

      window.matchMedia = function(query) {
        if (overrides.colorScheme && query.includes('prefers-color-scheme')) {
          return makeResult(query.includes(overrides.colorScheme), query);
        }
        if (overrides.reducedMotion && query.includes('prefers-reduced-motion')) {
          return makeResult(overrides.reducedMotion === 'reduce'
            ? query.includes('reduce') : query.includes('no-preference'), query);
        }
        if (overrides.forcedColors && query.includes('forced-colors')) {
          return makeResult(overrides.forcedColors === 'active'
            ? query.includes('active') : query.includes('none'), query);
        }
        return origMM(query);
      };

      ${colorScheme ? `document.documentElement.setAttribute('data-onecrawl-color-scheme', ${JSON.stringify(colorScheme)});` : ''}
      ${reducedMotion ? `document.documentElement.setAttribute('data-onecrawl-reduced-motion', ${JSON.stringify(reducedMotion)});` : ''}
      ${forcedColors ? `document.documentElement.setAttribute('data-onecrawl-forced-colors', ${JSON.stringify(forcedColors)});` : ''}

      ${media ? `
      // Inject print-media stylesheet override when media=print
      var existingStyle = document.getElementById('onecrawl-media-override');
      if (existingStyle) existingStyle.remove();
      if (${JSON.stringify(media)} === 'print') {
        var style = document.createElement('style');
        style.id = 'onecrawl-media-override';
        style.textContent = '@media screen { body { } } @media print { body { visibility: visible; } }';
        document.head.appendChild(style);
      }` : ''}

      return JSON.stringify({ emulatedMedia: overrides });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ emulatedMedia: emulated }));
    }
  });
}

module.exports = { register };
