'use strict';

/**
 * forms command — list all forms on the page with their fields.
 *
 * Usage:
 *   onecrawl-cli forms [--selector=<css>]
 *
 * Options:
 *   --selector  CSS selector to scope to a specific form
 *
 * For each form: action, method, and fields (name, type, required, value,
 * placeholder, options for select elements).
 * Output: JSON array of form descriptors.
 *
 * @module commands/forms
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'forms',
    description: 'list all forms on the page with their fields',
    usage: '[--selector=<css>]',
    action: formsAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function formsAction(args) {
  await withErrorHandling(async () => {
    const selector = args.selector || null;
    const selectorLiteral = selector ? JSON.stringify(selector) : 'null';

    const js = `(() => {
  var selector = ${selectorLiteral};
  var formEls = selector ? document.querySelectorAll(selector) : document.querySelectorAll('form');
  var results = [];
  for (var f = 0; f < formEls.length; f++) {
    var form = formEls[f];
    var descriptor = {
      action: form.action || '',
      method: (form.method || 'get').toUpperCase(),
      id: form.id || null,
      name: form.getAttribute('name') || null,
      fields: []
    };
    var inputs = form.querySelectorAll('input, select, textarea');
    for (var i = 0; i < inputs.length; i++) {
      var el = inputs[i];
      var tag = el.tagName.toLowerCase();
      var field = {
        tag: tag,
        name: el.name || null,
        type: el.type || (tag === 'textarea' ? 'textarea' : tag === 'select' ? 'select' : null),
        required: el.required || false,
        value: el.value || '',
        placeholder: el.placeholder || null
      };
      if (tag === 'select') {
        field.options = [];
        for (var o = 0; o < el.options.length; o++) {
          field.options.push({
            value: el.options[o].value,
            text: el.options[o].textContent.trim(),
            selected: el.options[o].selected
          });
        }
      }
      descriptor.fields.push(field);
    }
    results.push(descriptor);
  }
  return JSON.stringify(results);
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
  });
}

module.exports = { register };
