'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * user-agent command — override navigator.userAgent.
 *
 * Usage:
 *   onecrawl-cli user-agent <string>
 *   onecrawl-cli user-agent --reset
 *
 * @module commands/user-agent
 */

function register(registry) {
  registry.add({
    name: 'user-agent',
    description: 'override the browser user-agent string',
    usage: '<string> | --reset',
    action: userAgentAction,
  });
}

async function userAgentAction(args) {
  await withErrorHandling(async () => {
    const isReset = args.reset === true;
    const uaParts = args._.slice(1);
    const ua = uaParts.join(' ');

    if (!ua && !isReset) {
      console.error(
        'Usage: onecrawl-cli user-agent <string>\n' +
        '       onecrawl-cli user-agent --reset'
      );
      process.exit(1);
    }

    if (isReset) {
      const resetJs = `(() => {
        if (window.__onecrawl_orig_userAgent !== undefined) {
          Object.defineProperty(navigator, 'userAgent', {
            get: () => window.__onecrawl_orig_userAgent,
            configurable: true,
          });
          Object.defineProperty(navigator, 'appVersion', {
            get: () => window.__onecrawl_orig_appVersion,
            configurable: true,
          });
        }
        delete window.__onecrawl_user_agent;
        return JSON.stringify({ userAgent: 'reset' });
      })()`;

      await runSessionCommand({
        _: ['evaluate', resetJs],
        session: args.session,
      });
      console.log(JSON.stringify({ userAgent: 'reset' }));
      return;
    }

    const js = `(() => {
      var ua = ${JSON.stringify(ua)};
      if (window.__onecrawl_orig_userAgent === undefined) {
        window.__onecrawl_orig_userAgent = navigator.userAgent;
        window.__onecrawl_orig_appVersion = navigator.appVersion;
      }
      Object.defineProperty(navigator, 'userAgent', {
        get: () => ua,
        configurable: true,
      });
      Object.defineProperty(navigator, 'appVersion', {
        get: () => ua.replace(/^Mozilla\\//, ''),
        configurable: true,
      });
      window.__onecrawl_user_agent = ua;
      return JSON.stringify({ userAgent: ua });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ userAgent: ua }));
    }
  });
}

module.exports = { register };
