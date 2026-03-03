/**
 * Tool Registry — runtime catalog of OneCrawl CLI tools with schemas.
 */

import type { ToolSpec } from "./types.js";

const DEFAULT_TOOLS: ToolSpec[] = [
  { name: "navigate", description: "Navigate to a URL", usage: "goto <url>", positionalArgs: ["url"] },
  { name: "click", description: "Click an element by ref or selector", usage: "click <ref|selector>", positionalArgs: ["target"] },
  { name: "type", description: "Type text into an element", usage: 'type <ref|selector> "<text>"', positionalArgs: ["target", "text"] },
  { name: "find", description: "Find elements by strategy", usage: "find <strategy> <query>", positionalArgs: ["strategy", "query"] },
  { name: "wait-for", description: "Wait for a condition", usage: "wait-for <target> [timeout]", positionalArgs: ["target", "timeout"] },
  { name: "extract", description: "Extract structured data", usage: "extract [--selector=<css>] [--fields=<list>]", positionalArgs: [] },
  { name: "links", description: "Extract page links", usage: "links [--external] [--filter=<regex>]", positionalArgs: [] },
  { name: "table", description: "Extract HTML tables", usage: "table [selector] [--format=json|csv]", positionalArgs: ["selector"] },
  { name: "get", description: "Get a property value", usage: "get <property> [ref]", positionalArgs: ["property", "ref"] },
  { name: "is", description: "Check element state", usage: "is <state> <ref>", positionalArgs: ["state", "ref"] },
  { name: "assert", description: "Assert a condition", usage: "assert <condition> [ref] [value]", positionalArgs: ["condition", "ref", "value"] },
  { name: "scroll", description: "Scroll the page", usage: "scroll <direction> [pixels]", positionalArgs: ["direction", "pixels"] },
  { name: "screenshot-annotate", description: "Annotated screenshot", usage: "screenshot-annotate [file]", positionalArgs: ["file"] },
  { name: "session-info", description: "Session diagnostics", usage: "session-info", positionalArgs: [] },
  { name: "health-check", description: "Full diagnostic probe", usage: "health-check", positionalArgs: [] },
  { name: "auth", description: "Auth management", usage: "auth <action>", positionalArgs: ["action"] },
  { name: "viewport", description: "Set viewport size", usage: "viewport <width> <height>", positionalArgs: ["width", "height"] },
  { name: "device", description: "Emulate a device", usage: "device <name>", positionalArgs: ["name"] },
  { name: "route", description: "Intercept network requests", usage: "route <pattern> --action=<abort|mock>", positionalArgs: ["pattern"] },
  { name: "requests", description: "List captured requests", usage: "requests [--filter=<pattern>]", positionalArgs: [] },
  { name: "diff-snapshot", description: "DOM snapshot diff", usage: "diff-snapshot [--baseline=<file>]", positionalArgs: [] },
  { name: "diff-screenshot", description: "Visual diff", usage: "diff-screenshot [--baseline=<file>]", positionalArgs: [] },
  { name: "storage", description: "Manage localStorage/sessionStorage", usage: "storage <action> [key] [value]", positionalArgs: ["action", "key", "value"] },
  { name: "cookie", description: "Cookie management", usage: "cookie <action> [args...]", positionalArgs: ["action"] },
  { name: "keyboard", description: "Keyboard simulation", usage: "keyboard <action> <text>", positionalArgs: ["action", "text"] },
  { name: "clipboard", description: "Clipboard operations", usage: "clipboard <action> [text]", positionalArgs: ["action", "text"] },
  { name: "select", description: "Select dropdown option", usage: "select <ref|selector> <value>", positionalArgs: ["target", "value"] },
  { name: "hover", description: "Hover an element", usage: "hover <ref|selector>", positionalArgs: ["target"] },
  { name: "drag", description: "Drag and drop", usage: "drag <from> <to>", positionalArgs: ["from", "to"] },
  { name: "forms", description: "List page forms", usage: "forms [--selector=<css>]", positionalArgs: [] },
  { name: "tab", description: "Tab management", usage: "tab <action> [url|index]", positionalArgs: ["action", "urlOrIndex"] },
  { name: "console", description: "Browser console logs", usage: "console [--level=<filter>]", positionalArgs: [] },
  { name: "js-errors", description: "JavaScript errors", usage: "js-errors [--clear]", positionalArgs: [] },
];

export class ToolRegistry {
  private readonly tools = new Map<string, ToolSpec>();

  constructor(specs?: ToolSpec[]) {
    for (const spec of specs ?? DEFAULT_TOOLS) {
      this.tools.set(spec.name, spec);
    }
  }

  get(name: string): ToolSpec | undefined {
    return this.tools.get(name);
  }

  has(name: string): boolean {
    return this.tools.has(name);
  }

  list(): ToolSpec[] {
    return Array.from(this.tools.values());
  }

  /** Render tool documentation string for prompt injection. */
  toDocString(): string {
    return this.list()
      .map((t) => `- ${t.name}: ${t.description}\n  Usage: ${t.usage}`)
      .join("\n");
  }
}

export function buildToolRegistry(extra?: ToolSpec[]): ToolRegistry {
  const all = [...DEFAULT_TOOLS, ...(extra ?? [])];
  return new ToolRegistry(all);
}
