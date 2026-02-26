/**
 * Semantic Extractor
 * Pure functions for extracting interactive UI tools and links from HTML.
 * Uses regex-based extraction (no DOM parser dependency).
 */

import type { SemanticTool } from "../domain/semantic-tool.js";

/**
 * Convert a glob pattern to a RegExp.
 */
export function globToRegex(pattern: string): RegExp {
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*\*/g, "\0")
    .replace(/\*/g, "[^/]*")
    .replace(/\0/g, ".*")
    .replace(/\?/g, ".");
  return new RegExp(`^${escaped}$`);
}

/**
 * Check if a URL matches any of the given glob patterns.
 */
export function matchesPatterns(
  url: string,
  patterns: readonly string[],
): boolean {
  return patterns.some((p) => globToRegex(p).test(url));
}

// ---------------------------------------------------------------------------
// Internal regex helpers
// ---------------------------------------------------------------------------

const TAG_RE = (tag: string) =>
  new RegExp(`<${tag}\\b([^>]*)>([\\s\\S]*?)<\\/${tag}>`, "gi");
const SELF_CLOSING_RE = (tag: string) =>
  new RegExp(`<${tag}\\b([^>]*)\\/?>`, "gi");

function attr(html: string, name: string): string | undefined {
  const re = new RegExp(`${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)')`, "i");
  const m = re.exec(html);
  return m ? m[1] ?? m[2] : undefined;
}

function stripTags(html: string): string {
  return html.replace(/<[^>]*>/g, "").trim();
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_|_$/g, "")
    .slice(0, 60);
}

function inferInputType(
  htmlType: string | undefined,
): "string" | "number" | "boolean" {
  switch (htmlType?.toLowerCase()) {
    case "number":
    case "range":
      return "number";
    case "checkbox":
      return "boolean";
    default:
      return "string";
  }
}

// ---------------------------------------------------------------------------
// Tool extraction
// ---------------------------------------------------------------------------

function extractFormTools(html: string): SemanticTool[] {
  const tools: SemanticTool[] = [];
  const formRe = TAG_RE("form");
  let formMatch: RegExpExecArray | null;

  while ((formMatch = formRe.exec(html)) !== null) {
    const [, formAttrs, formBody] = formMatch;
    const formName =
      attr(formAttrs, "aria-label") ??
      attr(formAttrs, "name") ??
      attr(formAttrs, "id");
    if (!formName) continue;

    const properties: Record<
      string,
      { type: "string" | "number" | "boolean"; description?: string }
    > = {};
    const required: string[] = [];

    const inputRe = SELF_CLOSING_RE("input");
    let inputMatch: RegExpExecArray | null;
    while ((inputMatch = inputRe.exec(formBody)) !== null) {
      const [, inputAttrs] = inputMatch;
      const inputType = attr(inputAttrs, "type") ?? "text";
      if (inputType === "hidden" || inputType === "submit") continue;
      const name =
        attr(inputAttrs, "name") ??
        attr(inputAttrs, "id") ??
        attr(inputAttrs, "aria-label");
      if (!name) continue;
      properties[name] = {
        type: inferInputType(inputType),
        description:
          attr(inputAttrs, "placeholder") ??
          attr(inputAttrs, "aria-label") ??
          undefined,
      };
      if (
        attr(inputAttrs, "required") !== undefined ||
        inputAttrs.includes("required")
      ) {
        required.push(name);
      }
    }

    // Also extract <textarea> and <select>
    const textareaRe = TAG_RE("textarea");
    let taMatch: RegExpExecArray | null;
    while ((taMatch = textareaRe.exec(formBody)) !== null) {
      const name =
        attr(taMatch[1], "name") ??
        attr(taMatch[1], "id") ??
        attr(taMatch[1], "aria-label");
      if (name) {
        properties[name] = {
          type: "string",
          description: attr(taMatch[1], "placeholder") ?? undefined,
        };
      }
    }

    const selectRe = TAG_RE("select");
    let selMatch: RegExpExecArray | null;
    while ((selMatch = selectRe.exec(formBody)) !== null) {
      const name =
        attr(selMatch[1], "name") ??
        attr(selMatch[1], "id") ??
        attr(selMatch[1], "aria-label");
      if (name) {
        properties[name] = { type: "string" };
      }
    }

    if (Object.keys(properties).length === 0) continue;

    tools.push({
      name: `form_${slugify(formName)}`,
      description: `Form: ${formName}`,
      inputSchema: {
        type: "object" as const,
        properties,
        ...(required.length > 0 ? { required } : {}),
      },
      confidence: 0.9,
      category: "form",
    });
  }

  return tools;
}

function extractSearchTools(html: string): SemanticTool[] {
  const tools: SemanticTool[] = [];
  const inputRe = SELF_CLOSING_RE("input");
  let m: RegExpExecArray | null;

  while ((m = inputRe.exec(html)) !== null) {
    const [, attrs] = m;
    const type = attr(attrs, "type") ?? "text";
    const role = attr(attrs, "role");
    if (type !== "search" && role !== "search") continue;

    const label =
      attr(attrs, "aria-label") ??
      attr(attrs, "name") ??
      attr(attrs, "id") ??
      "query";
    const fieldName =
      attr(attrs, "name") ??
      attr(attrs, "id") ??
      attr(attrs, "aria-label") ??
      "query";
    const placeholder = attr(attrs, "placeholder");

    tools.push({
      name: `search_${slugify(label)}`,
      description: placeholder
        ? `Search: ${placeholder}`
        : `Search input: ${label}`,
      inputSchema: {
        type: "object" as const,
        properties: {
          [fieldName]: {
            type: "string" as const,
            description: placeholder ?? "Search query",
          },
        },
        required: [fieldName],
      },
      confidence: 0.85,
      category: "search",
    });
  }

  return tools;
}

function extractButtonTools(html: string): SemanticTool[] {
  const tools: SemanticTool[] = [];
  const btnRe = TAG_RE("button");
  let m: RegExpExecArray | null;

  while ((m = btnRe.exec(html)) !== null) {
    const [, attrs, body] = m;
    const type = attr(attrs, "type");
    if (type === "submit" || type === "reset") continue;

    const label =
      attr(attrs, "aria-label") ?? stripTags(body);
    if (!label || label.length === 0) continue;

    tools.push({
      name: `button_${slugify(label)}`,
      description: `Button: ${label}`,
      inputSchema: { type: "object" as const, properties: {} },
      confidence: 0.7,
      category: "button",
    });
  }

  return tools;
}

function extractNavTools(html: string): SemanticTool[] {
  const tools: SemanticTool[] = [];
  const navRe = TAG_RE("nav");
  let navMatch: RegExpExecArray | null;

  while ((navMatch = navRe.exec(html)) !== null) {
    const [, navAttrs, navBody] = navMatch;
    const navLabel =
      attr(navAttrs, "aria-label") ?? attr(navAttrs, "id") ?? "navigation";

    const linkRe = TAG_RE("a");
    let linkMatch: RegExpExecArray | null;
    const items: string[] = [];

    while ((linkMatch = linkRe.exec(navBody)) !== null) {
      const text =
        attr(linkMatch[1], "aria-label") ?? stripTags(linkMatch[2]);
      if (text) items.push(text);
    }

    if (items.length === 0) continue;

    tools.push({
      name: `nav_${slugify(navLabel)}`,
      description: `Navigation: ${navLabel} (${items.length} items)`,
      inputSchema: {
        type: "object" as const,
        properties: {
          item: {
            type: "string" as const,
            description: `One of: ${items.join(", ")}`,
          },
        },
        required: ["item"],
      },
      confidence: 0.75,
      category: "navigation",
    });
  }

  return tools;
}

/**
 * Extract interactive UI tools (forms, buttons, search inputs, nav links) from HTML.
 */
export function extractToolsFromHTML(
  html: string,
  _url: string,
): SemanticTool[] {
  const tools: SemanticTool[] = [
    ...extractFormTools(html),
    ...extractSearchTools(html),
    ...extractButtonTools(html),
    ...extractNavTools(html),
  ];

  // Deduplicate by name
  const seen = new Set<string>();
  return tools.filter((t) => {
    if (seen.has(t.name)) return false;
    seen.add(t.name);
    return true;
  });
}

/**
 * Extract same-origin internal links from HTML.
 */
export function extractInternalLinks(
  html: string,
  baseUrl: string,
): string[] {
  const base = new URL(baseUrl);
  const linkRe = /<a\b[^>]*\bhref\s*=\s*(?:"([^"]*)"|'([^']*)')/gi;
  const urls = new Set<string>();
  let m: RegExpExecArray | null;

  while ((m = linkRe.exec(html)) !== null) {
    const href = m[1] ?? m[2];
    if (!href || href.startsWith("#") || href.startsWith("javascript:") || href.startsWith("mailto:")) {
      continue;
    }

    try {
      const resolved = new URL(href, baseUrl);
      if (resolved.origin !== base.origin) continue;
      resolved.hash = "";
      urls.add(resolved.href);
    } catch {
      // invalid URL, skip
    }
  }

  return [...urls];
}
