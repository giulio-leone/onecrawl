'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * timezone command — override the browser timezone.
 *
 * Usage:
 *   onecrawl-cli timezone <iana-timezone>
 *   onecrawl-cli timezone America/New_York
 *   onecrawl-cli timezone --reset
 *
 * @module commands/timezone
 */

function register(registry) {
  registry.add({
    name: 'timezone',
    description: 'override the browser timezone via Intl.DateTimeFormat',
    usage: '<iana-timezone> | --reset',
    action: timezoneAction,
  });
}

async function timezoneAction(args) {
  await withErrorHandling(async () => {
    const isReset = args.reset === true;
    const tz = args._[1];

    if (!tz && !isReset) {
      console.error(
        'Usage: onecrawl-cli timezone <iana-timezone>\n' +
        '       onecrawl-cli timezone America/New_York\n' +
        '       onecrawl-cli timezone --reset'
      );
      process.exit(1);
    }

    if (isReset) {
      const resetJs = `(() => {
        if (window.__onecrawl_orig_date) {
          window.Date = window.__onecrawl_orig_date;
        }
        if (window.__onecrawl_orig_DateTimeFormat) {
          Intl.DateTimeFormat = window.__onecrawl_orig_DateTimeFormat;
        }
        if (window.__onecrawl_orig_toLocaleString) {
          Date.prototype.toLocaleString = window.__onecrawl_orig_toLocaleString;
          Date.prototype.toLocaleDateString = window.__onecrawl_orig_toLocaleDateString;
          Date.prototype.toLocaleTimeString = window.__onecrawl_orig_toLocaleTimeString;
        }
        delete window.__onecrawl_timezone;
        return JSON.stringify({ timezone: 'reset' });
      })()`;

      await runSessionCommand({
        _: ['evaluate', resetJs],
        session: args.session,
      });
      console.log(JSON.stringify({ timezone: 'reset' }));
      return;
    }

    const js = `(() => {
      const tz = ${JSON.stringify(tz)};
      try {
        Intl.DateTimeFormat(undefined, { timeZone: tz });
      } catch (e) {
        throw new Error('Invalid timezone: ' + tz);
      }

      if (!window.__onecrawl_orig_date) {
        window.__onecrawl_orig_date = Date;
      }
      if (!window.__onecrawl_orig_DateTimeFormat) {
        window.__onecrawl_orig_DateTimeFormat = Intl.DateTimeFormat;
      }
      const OrigDTF = window.__onecrawl_orig_DateTimeFormat;

      Intl.DateTimeFormat = function(locale, opts) {
        return new OrigDTF(locale, Object.assign({}, opts, { timeZone: tz }));
      };
      Intl.DateTimeFormat.prototype = OrigDTF.prototype;
      Intl.DateTimeFormat.supportedLocalesOf = OrigDTF.supportedLocalesOf;

      // Override Date.prototype locale methods
      if (!window.__onecrawl_orig_toLocaleString) {
        window.__onecrawl_orig_toLocaleString = Date.prototype.toLocaleString;
        window.__onecrawl_orig_toLocaleDateString = Date.prototype.toLocaleDateString;
        window.__onecrawl_orig_toLocaleTimeString = Date.prototype.toLocaleTimeString;
      }
      var origTLS = window.__onecrawl_orig_toLocaleString;
      var origTLDS = window.__onecrawl_orig_toLocaleDateString;
      var origTLTS = window.__onecrawl_orig_toLocaleTimeString;

      Date.prototype.toLocaleString = function(locale, opts) {
        return origTLS.call(this, locale, Object.assign({}, opts, { timeZone: tz }));
      };
      Date.prototype.toLocaleDateString = function(locale, opts) {
        return origTLDS.call(this, locale, Object.assign({}, opts, { timeZone: tz }));
      };
      Date.prototype.toLocaleTimeString = function(locale, opts) {
        return origTLTS.call(this, locale, Object.assign({}, opts, { timeZone: tz }));
      };

      window.__onecrawl_timezone = tz;
      return JSON.stringify({ timezone: tz });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ timezone: tz }));
    }
  });
}

module.exports = { register };
