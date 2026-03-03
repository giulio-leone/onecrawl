'use strict';

/**
 * trace command — collect performance timing data.
 *
 * Usage:
 *   onecrawl-cli trace start [--file=<path>]
 *   onecrawl-cli trace stop
 *
 * Uses performance.mark / performance.measure and PerformanceObserver.
 *
 * @module commands/trace
 */

const fs = require('fs');
const path = require('path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'trace',
    description: 'collect performance timing data',
    usage: 'start [--file=<path>] | stop',
    action: traceAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function traceAction(args) {
  await withErrorHandling(async () => {
    const subcommand = args._[1];

    if (!subcommand || !['start', 'stop'].includes(subcommand)) {
      console.error('Usage: onecrawl-cli trace start [--file=<path>]\n       onecrawl-cli trace stop');
      process.exit(1);
    }

    if (subcommand === 'start') {
      const js = `(() => {
        window.__onecrawl_trace = { events: [], startTime: performance.now() };

        if (!window.__onecrawl_trace_observer) {
          const obs = new PerformanceObserver(function(list) {
            if (!window.__onecrawl_trace) return;
            for (const entry of list.getEntries()) {
              window.__onecrawl_trace.events.push({
                name: entry.name,
                entryType: entry.entryType,
                startTime: entry.startTime,
                duration: entry.duration,
                timestamp: Date.now()
              });
            }
          });
          obs.observe({ entryTypes: ['mark', 'measure', 'resource', 'navigation', 'paint', 'longtask'] });
          window.__onecrawl_trace_observer = obs;
        }

        const existing = performance.getEntriesByType('navigation')
          .concat(performance.getEntriesByType('resource'))
          .concat(performance.getEntriesByType('paint'));
        for (const entry of existing) {
          window.__onecrawl_trace.events.push({
            name: entry.name,
            entryType: entry.entryType,
            startTime: entry.startTime,
            duration: entry.duration,
            timestamp: Date.now()
          });
        }

        return JSON.stringify({ tracing: true });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      try {
        console.log(JSON.stringify(JSON.parse(result.text)));
      } catch {
        console.log(result.text);
      }
    } else {
      // stop
      const js = `(() => {
        if (!window.__onecrawl_trace) {
          return JSON.stringify({ error: 'Trace not started' });
        }
        const events = window.__onecrawl_trace.events;
        const count = events.length;
        if (window.__onecrawl_trace_observer) {
          window.__onecrawl_trace_observer.disconnect();
          window.__onecrawl_trace_observer = null;
        }
        window.__onecrawl_trace = null;
        return JSON.stringify({ _events: events, events: count });
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

      const filePath = args.file || path.resolve(process.cwd(), 'trace.json');
      const traceData = {
        traceEvents: parsed._events.map((e, i) => ({
          pid: 1,
          tid: 1,
          ts: Math.round(e.startTime * 1000),
          dur: Math.round((e.duration || 0) * 1000),
          ph: e.duration > 0 ? 'X' : 'I',
          cat: e.entryType,
          name: e.name,
          args: {}
        }))
      };
      fs.writeFileSync(filePath, JSON.stringify(traceData, null, 2), 'utf8');
      console.log(JSON.stringify({ saved: filePath, events: parsed.events }));
    }
  });
}

module.exports = { register };
