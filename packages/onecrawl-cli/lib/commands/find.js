'use strict';

/**
 * find command — locate elements on the active page using various strategies.
 *
 * Usage:
 *   onecrawl-cli find <strategy> <query>
 *
 * Strategies: role, text, label, placeholder, testid, css
 * Returns JSON array of matching elements with ref numbers, text content,
 * tag name, and visibility state. Assigns data-oncrawl-ref attributes to
 * matched elements for use with get/is/assert commands.
 *
 * @module commands/find
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const VALID_STRATEGIES = ['role', 'text', 'label', 'placeholder', 'testid', 'css'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'find',
    description: 'find elements by strategy (role/text/label/placeholder/testid/css)',
    usage: '<strategy> <query>',
    action: findAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function findAction(args) {
  await withErrorHandling(async () => {
    const strategy = args._[1];
    const query = args._[2];

    if (!strategy || !VALID_STRATEGIES.includes(strategy) || !query) {
      console.error(
        `Usage: onecrawl-cli find <strategy> <query>\n` +
        `Strategies: ${VALID_STRATEGIES.join(', ')}`
      );
      process.exit(1);
    }

    const js = `(() => {
      document.querySelectorAll('[data-oncrawl-ref]').forEach(el => el.removeAttribute('data-oncrawl-ref'));

      const strategy = ${JSON.stringify(strategy)};
      const query = ${JSON.stringify(query)};
      let elements = [];

      switch (strategy) {
        case 'css':
          elements = Array.from(document.querySelectorAll(query));
          break;

        case 'text': {
          const all = document.body.querySelectorAll('*');
          for (const el of all) {
            const t = (el.innerText || el.textContent || '').trim();
            if (t.includes(query)) {
              const childMatch = Array.from(el.children).some(c =>
                ((c.innerText || c.textContent || '').trim()).includes(query)
              );
              if (!childMatch) elements.push(el);
            }
          }
          break;
        }

        case 'role': {
          const roleMap = {
            button: 'button,input[type=button],input[type=submit],input[type=reset]',
            link: 'a[href]',
            textbox: 'input[type=text],input:not([type]),textarea',
            checkbox: 'input[type=checkbox]',
            radio: 'input[type=radio]',
            heading: 'h1,h2,h3,h4,h5,h6',
            img: 'img[alt]',
            list: 'ul,ol',
            listitem: 'li',
            navigation: 'nav',
            main: 'main',
            banner: 'header',
            contentinfo: 'footer',
          };
          document.querySelectorAll('[role]').forEach(el => {
            if (el.getAttribute('role') === query) elements.push(el);
          });
          if (roleMap[query]) {
            Array.from(document.querySelectorAll(roleMap[query])).forEach(el => {
              if (!elements.includes(el)) elements.push(el);
            });
          }
          break;
        }

        case 'label': {
          document.querySelectorAll('label').forEach(lbl => {
            if (lbl.textContent.trim().includes(query)) {
              const target = lbl.htmlFor
                ? document.getElementById(lbl.htmlFor)
                : lbl.querySelector('input,select,textarea');
              if (target && !elements.includes(target)) elements.push(target);
            }
          });
          document.querySelectorAll('[aria-label]').forEach(el => {
            if (el.getAttribute('aria-label').includes(query) && !elements.includes(el)) {
              elements.push(el);
            }
          });
          break;
        }

        case 'placeholder': {
          document.querySelectorAll('[placeholder]').forEach(el => {
            if (el.getAttribute('placeholder').includes(query)) elements.push(el);
          });
          break;
        }

        case 'testid': {
          document.querySelectorAll('[data-testid]').forEach(el => {
            if (el.getAttribute('data-testid') === query) elements.push(el);
          });
          break;
        }
      }

      const results = elements.map((el, i) => {
        const ref = i + 1;
        el.setAttribute('data-oncrawl-ref', String(ref));
        const rect = el.getBoundingClientRect();
        const style = getComputedStyle(el);
        return {
          ref: ref,
          tag: el.tagName.toLowerCase(),
          text: (el.textContent || '').trim().slice(0, 100),
          visible: rect.width > 0 && rect.height > 0 &&
                   style.visibility !== 'hidden' && style.display !== 'none',
        };
      });

      return JSON.stringify(results);
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      const parsed = JSON.parse(result.text);
      console.log(JSON.stringify(parsed));
    } catch {
      console.log(result.text);
    }
  });
}

module.exports = { register };
