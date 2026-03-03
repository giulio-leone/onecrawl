'use strict';

/**
 * recording command — start / stop / restart page recording via MediaRecorder.
 *
 * Usage:
 *   onecrawl-cli recording start [--file=<path>] [--format=webm|mp4]
 *   onecrawl-cli recording stop
 *   onecrawl-cli recording restart
 *
 * Uses the MediaRecorder API via evaluate.  May not be available in headless
 * mode — a meaningful error is returned in that case.
 *
 * @module commands/recording
 */

const fs = require('node:fs');
const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

const VALID_ACTIONS = ['start', 'stop', 'restart'];
const VALID_FORMATS = ['webm', 'mp4'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'recording',
    description: 'start, stop or restart page recording',
    usage: 'start|stop|restart [--file=<path>] [--format=webm|mp4]',
    action: recordingAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function recordingAction(args) {
  await withErrorHandling(async () => {
    const sub = args._[1];

    if (!sub || !VALID_ACTIONS.includes(sub)) {
      console.error(
        'Usage: onecrawl-cli recording start|stop|restart [--file=<path>] [--format=webm|mp4]'
      );
      process.exit(1);
    }

    const format = args.format || 'webm';
    if (!VALID_FORMATS.includes(format)) {
      console.error(`Invalid format "${format}". Valid: ${VALID_FORMATS.join(', ')}`);
      process.exit(1);
    }

    const mimeType = format === 'mp4' ? 'video/mp4' : 'video/webm';

    if (sub === 'start' || sub === 'restart') {
      const js = `(async () => {
        try {
          // Stop any existing recording
          if (window.__onecrawl_recorder && window.__onecrawl_recorder.state !== 'inactive') {
            window.__onecrawl_recorder.stop();
          }

          const canvas = document.createElement('canvas');
          canvas.width = window.innerWidth;
          canvas.height = window.innerHeight;
          const ctx = canvas.getContext('2d');
          const stream = canvas.captureStream(30);

          if (!MediaRecorder.isTypeSupported(${JSON.stringify(mimeType)})) {
            return JSON.stringify({ error: true, message: 'MediaRecorder does not support ${mimeType} in this browser' });
          }

          const recorder = new MediaRecorder(stream, { mimeType: ${JSON.stringify(mimeType)} });
          window.__onecrawl_recorder = recorder;
          window.__onecrawl_recording_chunks = [];
          window.__onecrawl_recording_start = Date.now();

          recorder.ondataavailable = function(e) {
            if (e.data.size > 0) window.__onecrawl_recording_chunks.push(e.data);
          };

          recorder.start(1000);

          return JSON.stringify({
            recording: true,
            format: ${JSON.stringify(format)},
            action: ${JSON.stringify(sub)},
          });
        } catch(e) {
          return JSON.stringify({
            error: true,
            message: 'Recording not supported: ' + e.message,
          });
        }
      })()`;

      const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
      console.log(result.text);
      return;
    }

    if (sub === 'stop') {
      const file = args.file || `recording-${Date.now()}.${format}`;
      const filePath = path.resolve(file);

      const js = `(async () => {
        if (!window.__onecrawl_recorder) {
          return JSON.stringify({ error: true, message: 'No recording in progress' });
        }

        return new Promise(function(resolve) {
          const recorder = window.__onecrawl_recorder;
          recorder.onstop = function() {
            const blob = new Blob(window.__onecrawl_recording_chunks, { type: ${JSON.stringify(mimeType)} });
            const reader = new FileReader();
            reader.onloadend = function() {
              const base64 = reader.result.split(',')[1] || '';
              const duration = Date.now() - (window.__onecrawl_recording_start || Date.now());
              resolve(JSON.stringify({
                stopped: true,
                format: ${JSON.stringify(format)},
                duration: duration,
                size: blob.size,
                data: base64,
              }));
            };
            reader.readAsDataURL(blob);
          };
          recorder.stop();
        });
      })()`;

      const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
      const parsed = JSON.parse(result.text);

      if (parsed.error) {
        console.log(result.text);
        return;
      }

      if (parsed.data) {
        fs.writeFileSync(filePath, Buffer.from(parsed.data, 'base64'));
        delete parsed.data;
        parsed.saved = filePath;
      }

      console.log(JSON.stringify(parsed));
    }
  });
}

module.exports = { register };
