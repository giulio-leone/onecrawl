/**
 * Ghost Cursor init script — injected into the browser to add human-like
 * mouse movement before click events dispatched by automation.
 *
 * Self-contained (no require/import). Runs inside browser context.
 * Loaded via browser.initScript when GHOST_CURSOR_ENABLED=true.
 *
 * How it works:
 *  - Overrides Element.prototype.click
 *  - Detects automation clicks (isTrusted === false on the triggering path)
 *  - Generates a Bezier-curve mouse path to the target element
 *  - Dispatches mousemove events along the path with realistic timing
 *  - Then fires mousedown → mouseup → click with correct coordinates
 */

(function () {
  'use strict';

  // ── Global mouse position tracker ───────────────────────────────
  window.__mouseX = Math.floor(Math.random() * (window.innerWidth || 1024));
  window.__mouseY = Math.floor(Math.random() * (window.innerHeight || 768));

  document.addEventListener('mousemove', function (e) {
    if (e.isTrusted) {
      window.__mouseX = e.clientX;
      window.__mouseY = e.clientY;
    }
  }, true);

  // ── Math helpers ────────────────────────────────────────────────

  function rand(min, max) {
    return min + Math.random() * (max - min);
  }

  function clamp(v, lo, hi) {
    return Math.max(lo, Math.min(hi, v));
  }

  // Ease-in-out curve: slow at edges, fast in middle
  function easeInOut(t) {
    return t < 0.5
      ? 4 * t * t * t
      : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }

  // ── Bezier curve generation ─────────────────────────────────────

  /**
   * Evaluate a generic Bezier curve at parameter t using De Casteljau.
   * @param {Array<{x:number,y:number}>} pts - control points
   * @param {number} t - parameter [0,1]
   */
  function bezier(pts, t) {
    let work = pts.map(function (p) { return { x: p.x, y: p.y }; });
    while (work.length > 1) {
      var next = [];
      for (var i = 0; i < work.length - 1; i++) {
        next.push({
          x: work[i].x + (work[i + 1].x - work[i].x) * t,
          y: work[i].y + (work[i + 1].y - work[i].y) * t,
        });
      }
      work = next;
    }
    return work[0];
  }

  /**
   * Build a path of screen-space points along a Bezier curve
   * from `start` to `end` with 2-5 random control points.
   */
  function buildPath(start, end, steps) {
    var numCtrl = 2 + Math.floor(Math.random() * 4); // 2-5
    var dx = end.x - start.x;
    var dy = end.y - start.y;

    var ctrlPts = [start];
    for (var c = 0; c < numCtrl; c++) {
      var frac = (c + 1) / (numCtrl + 1);
      ctrlPts.push({
        x: start.x + dx * frac + rand(-0.3, 0.3) * Math.abs(dx || 60),
        y: start.y + dy * frac + rand(-0.3, 0.3) * Math.abs(dy || 60),
      });
    }
    ctrlPts.push(end);

    var path = [];
    for (var i = 0; i <= steps; i++) {
      var t = i / steps;
      var eased = easeInOut(t);
      var pt = bezier(ctrlPts, eased);
      // Add natural jitter (±1-2px), less at endpoints
      var jitterScale = Math.sin(Math.PI * t); // 0 at edges, 1 in middle
      pt.x += rand(-2, 2) * jitterScale;
      pt.y += rand(-2, 2) * jitterScale;
      path.push({
        x: Math.round(clamp(pt.x, 0, window.innerWidth - 1)),
        y: Math.round(clamp(pt.y, 0, window.innerHeight - 1)),
      });
    }
    return path;
  }

  // ── Dispatch helpers ────────────────────────────────────────────

  function dispatchMouse(target, type, x, y, opts) {
    var evt = new MouseEvent(type, Object.assign({
      bubbles: true,
      cancelable: true,
      view: window,
      clientX: x,
      clientY: y,
      screenX: x + (window.screenX || 0),
      screenY: y + (window.screenY || 0),
    }, opts || {}));
    target.dispatchEvent(evt);
  }

  // ── Sleep utility (browser) ─────────────────────────────────────

  function sleep(ms) {
    return new Promise(function (resolve) { setTimeout(resolve, ms); });
  }

  // ── Override Element.prototype.click ────────────────────────────

  var _origClick = Element.prototype.click;

  // Guard to prevent re-entrant interception
  var _ghostActive = false;

  Element.prototype.click = function ghostClick() {
    var el = this;

    // Only intercept automation-triggered clicks (synchronous call path).
    // If we're already inside a ghost sequence, delegate to native.
    if (_ghostActive) {
      return _origClick.call(el);
    }

    // Heuristic: Element.prototype.click() is called programmatically
    // (isTrusted will be false on the resulting event). Real user clicks
    // go through the browser's event pipeline and never hit this override.
    // We wrap the async work in a microtask-safe way.
    _ghostActive = true;

    (async function () {
      try {
        var rect = el.getBoundingClientRect();
        if (!rect || rect.width === 0 || rect.height === 0) {
          _origClick.call(el);
          return;
        }

        // Random target point within element (not always center)
        var targetX = rect.left + rect.width * rand(0.3, 0.7);
        var targetY = rect.top + rect.height * rand(0.3, 0.7);

        var startX = window.__mouseX;
        var startY = window.__mouseY;
        var dist = Math.hypot(targetX - startX, targetY - startY);

        // Scale step count by distance (min 8, max 40)
        var steps = Math.max(8, Math.min(40, Math.round(dist / 15)));
        var path = buildPath(
          { x: startX, y: startY },
          { x: targetX, y: targetY },
          steps
        );

        // Dispatch mousemove events along the path
        for (var i = 0; i < path.length; i++) {
          var pt = path[i];
          var elAtPt = document.elementFromPoint(pt.x, pt.y) || el;
          dispatchMouse(elAtPt, 'mousemove', pt.x, pt.y);
          window.__mouseX = pt.x;
          window.__mouseY = pt.y;
          // Variable delay: 5-25ms per step
          await sleep(Math.floor(rand(5, 25)));
        }

        // Small pause before clicking (human reaction)
        await sleep(Math.floor(rand(20, 80)));

        // Fire mousedown → mouseup → click at final position
        var cx = path[path.length - 1].x;
        var cy = path[path.length - 1].y;
        var btnOpts = { button: 0, buttons: 1 };

        dispatchMouse(el, 'mousedown', cx, cy, btnOpts);
        await sleep(Math.floor(rand(30, 90)));
        dispatchMouse(el, 'mouseup', cx, cy, { button: 0, buttons: 0 });
        await sleep(Math.floor(rand(5, 15)));
        dispatchMouse(el, 'click', cx, cy, { button: 0, buttons: 0 });
      } catch (_e) {
        // Fallback to native click on any error
        _origClick.call(el);
      } finally {
        _ghostActive = false;
      }
    })();
  };
})();
