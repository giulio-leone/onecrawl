'use strict';

const fs = require('fs');
const path = require('path');

/**
 * @typedef {Object} CommandDefinition
 * @property {string} name - Command name (used in argv)
 * @property {string} description - One-line description for help text
 * @property {string} usage - Usage string (e.g. '<direction> [pixels]')
 * @property {function(Object): Promise<void>} action - Handler receiving parsed minimist args
 */

/**
 * Registry of custom onecrawl commands.
 * Auto-discovers all .js files in lib/commands/ (except index.js)
 * and loads their exported { register } function.
 */
class CommandRegistry {
  constructor() {
    /** @type {Map<string, CommandDefinition>} */
    this._commands = new Map();
  }

  /**
   * Register a single command definition.
   * @param {CommandDefinition} def
   */
  add(def) {
    if (!def.name || typeof def.action !== 'function') {
      throw new Error(`Invalid command definition: name and action are required`);
    }
    if (this._commands.has(def.name)) {
      throw new Error(`Duplicate command: '${def.name}' is already registered`);
    }
    this._commands.set(def.name, def);
  }

  /**
   * Check if a command name is registered.
   * @param {string} name
   * @returns {boolean}
   */
  has(name) {
    return this._commands.has(name);
  }

  /**
   * Get a command definition by name.
   * @param {string} name
   * @returns {CommandDefinition|undefined}
   */
  get(name) {
    return this._commands.get(name);
  }

  /**
   * Return all registered command definitions.
   * @returns {CommandDefinition[]}
   */
  all() {
    return Array.from(this._commands.values());
  }
}

/**
 * Auto-discover and load all command files in this directory.
 * Each file must export: { register(registry) }
 *
 * @returns {CommandRegistry}
 */
function loadAllCommands() {
  const registry = new CommandRegistry();
  const commandsDir = __dirname;

  let files;
  try {
    files = fs.readdirSync(commandsDir);
  } catch (err) {
    console.error(`onecrawl: failed to read commands directory: ${err.message}`);
    process.exit(1);
  }

  for (const file of files) {
    if (file === 'index.js' || !file.endsWith('.js')) continue;

    const filePath = path.join(commandsDir, file);
    try {
      const mod = require(filePath);
      if (typeof mod.register !== 'function') {
        console.error(`onecrawl: command file '${file}' does not export register()`);
        continue;
      }
      mod.register(registry);
    } catch (err) {
      console.error(`onecrawl: failed to load command '${file}': ${err.message}`);
    }
  }

  return registry;
}

/**
 * Build a help block for all custom commands (appended to Playwright help).
 * @param {CommandRegistry} registry
 * @returns {string}
 */
function formatCustomHelp(registry) {
  const defs = registry.all();
  if (defs.length === 0) return '';

  const lines = ['\nOneCrawl commands:'];
  const maxLen = Math.max(...defs.map(d => {
    const full = d.usage ? `${d.name} ${d.usage}` : d.name;
    return full.length;
  }));

  for (const def of defs) {
    const full = def.usage ? `${def.name} ${def.usage}` : def.name;
    lines.push(`  ${full.padEnd(maxLen + 4)}${def.description}`);
  }
  return lines.join('\n');
}

module.exports = { CommandRegistry, loadAllCommands, formatCustomHelp };
