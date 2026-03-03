'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * locale command — override navigator.language and navigator.languages.
 *
 * Usage:
 *   onecrawl-cli locale <locale-code>
 *   onecrawl-cli locale it-IT
 *   onecrawl-cli locale --reset
 *
 * @module commands/locale
 */

function register(registry) {
  registry.add({
    name: 'locale',
    description: 'override navigator.language and navigator.languages',
    usage: '<locale-code> | --reset',
    action: localeAction,
  });
}

async function localeAction(args) {
  await withErrorHandling(async () => {
    const isReset = args.reset === true;
    const locale = args._[1];

    if (!locale && !isReset) {
      console.error(
        'Usage: onecrawl-cli locale <locale-code>\n' +
        '       onecrawl-cli locale it-IT\n' +
        '       onecrawl-cli locale --reset'
      );
      process.exit(1);
    }

    if (isReset) {
      const resetJs = `(() => {
        if (window.__onecrawl_orig_language !== undefined) {
          Object.defineProperty(navigator, 'language', {
            get: () => window.__onecrawl_orig_language,
            configurable: true,
          });
          Object.defineProperty(navigator, 'languages', {
            get: () => window.__onecrawl_orig_languages,
            configurable: true,
          });
        }
        delete window.__onecrawl_locale;
        return JSON.stringify({ locale: 'reset' });
      })()`;

      await runSessionCommand({
        _: ['evaluate', resetJs],
        session: args.session,
      });
      console.log(JSON.stringify({ locale: 'reset' }));
      return;
    }

    if (!/^[a-zA-Z]{2,3}(-[a-zA-Z0-9]{2,8})*$/.test(locale)) {
      console.error(`Invalid locale format: "${locale}"`);
      console.error('Expected format: language[-region], e.g. en-US, it-IT, zh-Hans-CN');
      process.exit(1);
    }

    const js = `(() => {
      var locale = ${JSON.stringify(locale)};
      if (window.__onecrawl_orig_language === undefined) {
        window.__onecrawl_orig_language = navigator.language;
        window.__onecrawl_orig_languages = navigator.languages ? Array.from(navigator.languages) : [navigator.language];
      }
      Object.defineProperty(navigator, 'language', {
        get: () => locale,
        configurable: true,
      });
      Object.defineProperty(navigator, 'languages', {
        get: () => [locale],
        configurable: true,
      });
      window.__onecrawl_locale = locale;
      return JSON.stringify({ locale: locale });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ locale }));
    }
  });
}

module.exports = { register };
