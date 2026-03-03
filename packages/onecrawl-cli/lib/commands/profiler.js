'use strict';

/**
 * profiler command — start and stop JavaScript profiling.
 *
 * Usage:
 *   onecrawl-cli profiler start
 *   onecrawl-cli profiler stop [--file=<path>]
 *
 * Uses console.profile / console.profileEnd and the Performance API
 * to collect profiling data.
 *
 * @module commands/profiler
 */

const fs = require('fs');
const path = require('path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'profiler',
    description: 'start or stop JavaScript profiling',
    usage: 'start | stop [--file=<path>]',
    action: profilerAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function profilerAction(args) {
  await withErrorHandling(async () => {
    const subcommand = args._[1];

    if (!subcommand || !['start', 'stop'].includes(subcommand)) {
      console.error('Usage: onecrawl-cli profiler start\n       onecrawl-cli profiler stop [--file=<path>]');
      process.exit(1);
    }

    if (subcommand === 'start') {
      const js = `(() => {
        if (window.__onecrawl_profiling) {
          return JSON.stringify({ error: 'Profiler already running' });
        }
        window.__onecrawl_profiling = true;
        window.__onecrawl_profile_start = performance.now();
        window.__onecrawl_profile_marks = [];

        if (typeof console.profile === 'function') {
          console.profile('onecrawl-profile');
        }

        var obs = new PerformanceObserver(function(list) {
          if (!window.__onecrawl_profiling) return;
          for (var entry of list.getEntries()) {
            window.__onecrawl_profile_marks.push({
              name: entry.name,
              entryType: entry.entryType,
              startTime: entry.startTime,
              duration: entry.duration
            });
          }
        });
        obs.observe({ entryTypes: ['measure', 'mark', 'longtask'] });
        window.__onecrawl_profile_observer = obs;

        return JSON.stringify({ profiling: true });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      let parsed;
      try {
        parsed = JSON.parse(result.text);
      } catch {
        console.log(result.text);
        return;
      }

      if (parsed.error) {
        console.error(parsed.error);
        process.exit(1);
      }
      console.log(JSON.stringify(parsed));
    } else {
      // stop
      const js = `(() => {
        if (!window.__onecrawl_profiling) {
          return JSON.stringify({ error: 'Profiler not started' });
        }

        if (typeof console.profileEnd === 'function') {
          console.profileEnd('onecrawl-profile');
        }

        if (window.__onecrawl_profile_observer) {
          window.__onecrawl_profile_observer.disconnect();
          window.__onecrawl_profile_observer = null;
        }

        var duration = performance.now() - (window.__onecrawl_profile_start || 0);
        var marks = window.__onecrawl_profile_marks || [];

        var resources = performance.getEntriesByType('resource').map(function(e) {
          return { name: e.name, entryType: e.entryType, startTime: e.startTime, duration: e.duration };
        });

        var profile = {
          duration: duration,
          marks: marks,
          resources: resources,
          timestamp: new Date().toISOString()
        };

        window.__onecrawl_profiling = false;
        window.__onecrawl_profile_marks = [];

        return JSON.stringify({ _profile: profile, entries: marks.length + resources.length });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      let parsed;
      try {
        parsed = JSON.parse(result.text);
      } catch {
        console.log(result.text);
        return;
      }

      if (parsed.error) {
        console.error(parsed.error);
        process.exit(1);
      }

      const filePath = args.file || path.resolve(process.cwd(), 'profile.json');
      fs.writeFileSync(filePath, JSON.stringify(parsed._profile, null, 2), 'utf8');
      console.log(JSON.stringify({ saved: filePath, entries: parsed.entries }));
    }
  });
}

module.exports = { register };
