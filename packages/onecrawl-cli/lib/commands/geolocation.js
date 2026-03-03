'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * geolocation command — override navigator.geolocation.
 *
 * Usage:
 *   onecrawl-cli geolocation <latitude> <longitude> [--accuracy=<meters>]
 *   onecrawl-cli geolocation --reset
 *
 * @module commands/geolocation
 */

function register(registry) {
  registry.add({
    name: 'geolocation',
    description: 'override browser geolocation (latitude, longitude, accuracy)',
    usage: '<latitude> <longitude> [--accuracy=<meters>] | --reset',
    action: geolocationAction,
  });
}

async function geolocationAction(args) {
  await withErrorHandling(async () => {
    const isReset = args.reset === true;
    const lat = parseFloat(args._[1]);
    const lng = parseFloat(args._[2]);
    const accuracy = parseFloat(args.accuracy) || 100;

    if (!isReset && (isNaN(lat) || isNaN(lng))) {
      console.error(
        'Usage: onecrawl-cli geolocation <latitude> <longitude> [--accuracy=<meters>]\n' +
        '       onecrawl-cli geolocation --reset'
      );
      process.exit(1);
    }

    if (isReset) {
      const resetJs = `(() => {
        if (window.__onecrawl_orig_getCurrentPosition) {
          navigator.geolocation.getCurrentPosition = window.__onecrawl_orig_getCurrentPosition;
          navigator.geolocation.watchPosition = window.__onecrawl_orig_watchPosition;
          navigator.geolocation.clearWatch = window.__onecrawl_orig_clearWatch;
        }
        delete window.__onecrawl_geolocation;
        return JSON.stringify({ geolocation: 'reset' });
      })()`;

      await runSessionCommand({
        _: ['evaluate', resetJs],
        session: args.session,
      });
      console.log(JSON.stringify({ geolocation: 'reset' }));
      return;
    }

    if (lat < -90 || lat > 90) {
      console.error(`Invalid latitude: ${lat}. Must be between -90 and 90.`);
      process.exit(1);
    }
    if (lng < -180 || lng > 180) {
      console.error(`Invalid longitude: ${lng}. Must be between -180 and 180.`);
      process.exit(1);
    }
    if (accuracy <= 0) {
      console.error('Accuracy must be a positive number of meters.');
      process.exit(1);
    }

    const js = `(() => {
      var pos = {
        coords: {
          latitude: ${lat},
          longitude: ${lng},
          accuracy: ${accuracy},
          altitude: null,
          altitudeAccuracy: null,
          heading: null,
          speed: null,
        },
        timestamp: Date.now(),
      };

      // Save originals for reset
      if (!window.__onecrawl_orig_getCurrentPosition) {
        window.__onecrawl_orig_getCurrentPosition = navigator.geolocation.getCurrentPosition.bind(navigator.geolocation);
        window.__onecrawl_orig_watchPosition = navigator.geolocation.watchPosition.bind(navigator.geolocation);
        window.__onecrawl_orig_clearWatch = navigator.geolocation.clearWatch.bind(navigator.geolocation);
      }

      navigator.geolocation.getCurrentPosition = function(success) { success(pos); };
      navigator.geolocation.watchPosition = function(success) { success(pos); return 0; };
      navigator.geolocation.clearWatch = function() {};

      window.__onecrawl_geolocation = { latitude: ${lat}, longitude: ${lng}, accuracy: ${accuracy} };
      return JSON.stringify({ geolocation: { latitude: ${lat}, longitude: ${lng}, accuracy: ${accuracy} } });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ geolocation: { latitude: lat, longitude: lng, accuracy } }));
    }
  });
}

module.exports = { register };
