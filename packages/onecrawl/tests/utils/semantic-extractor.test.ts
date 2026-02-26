/**
 * Semantic Extractor — pure function tests.
 */

import { describe, it, expect } from "vitest";
import {
  globToRegex,
  matchesPatterns,
  extractToolsFromHTML,
  extractInternalLinks,
} from "../../src/utils/semantic-extractor.js";

describe("globToRegex", () => {
  it("converts * to match path segment", () => {
    const re = globToRegex("*.txt");
    expect(re.test("file.txt")).toBe(true);
    expect(re.test("dir/file.txt")).toBe(false);
  });

  it("converts ** to match across segments", () => {
    const re = globToRegex("**/file.txt");
    // ** expands to .* which requires at least a / separator
    expect(re.test("dir/file.txt")).toBe(true);
    expect(re.test("a/b/file.txt")).toBe(true);
  });

  it("converts ? to match single char", () => {
    const re = globToRegex("file?.txt");
    expect(re.test("file1.txt")).toBe(true);
    expect(re.test("file.txt")).toBe(false);
  });

  it("escapes special regex chars", () => {
    const re = globToRegex("file.name+test");
    expect(re.test("file.name+test")).toBe(true);
    expect(re.test("fileXnameXtest")).toBe(false);
  });
});

describe("matchesPatterns", () => {
  it("matches URL against patterns", () => {
    expect(matchesPatterns("https://example.com/docs/api", ["**/docs/**"])).toBe(true);
  });

  it("returns false when no patterns match", () => {
    expect(matchesPatterns("https://example.com/about", ["**/docs/**"])).toBe(false);
  });

  it("returns true when any pattern matches", () => {
    expect(
      matchesPatterns("https://example.com/about", ["**/docs/**", "**/about"]),
    ).toBe(true);
  });

  it("returns false for empty patterns", () => {
    expect(matchesPatterns("https://example.com/x", [])).toBe(false);
  });
});

describe("extractToolsFromHTML", () => {
  it("extracts form tools", () => {
    const html = `
      <form name="login">
        <input name="username" type="text" placeholder="Username" required/>
        <input name="password" type="password" placeholder="Password" required/>
      </form>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    expect(tools.length).toBeGreaterThanOrEqual(1);
    const formTool = tools.find((t) => t.category === "form");
    expect(formTool).toBeDefined();
    expect(formTool!.name).toContain("login");
    expect(formTool!.inputSchema.properties).toHaveProperty("username");
    expect(formTool!.inputSchema.properties).toHaveProperty("password");
    expect(formTool!.inputSchema.required).toContain("username");
  });

  it("extracts search input tools", () => {
    const html = `<input type="search" name="q" aria-label="Search" placeholder="Search the site"/>`;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const searchTool = tools.find((t) => t.category === "search");
    expect(searchTool).toBeDefined();
    expect(searchTool!.inputSchema.properties).toHaveProperty("q");
  });

  it("extracts button tools", () => {
    const html = `<button aria-label="Download PDF">Download</button>`;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const btnTool = tools.find((t) => t.category === "button");
    expect(btnTool).toBeDefined();
    expect(btnTool!.name).toContain("download");
  });

  it("skips submit/reset buttons", () => {
    const html = `
      <button type="submit">Submit</button>
      <button type="reset">Reset</button>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const btnTools = tools.filter((t) => t.category === "button");
    expect(btnTools).toHaveLength(0);
  });

  it("extracts navigation tools", () => {
    const html = `
      <nav aria-label="Main menu">
        <a href="/home">Home</a>
        <a href="/about">About</a>
        <a href="/contact">Contact</a>
      </nav>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const navTool = tools.find((t) => t.category === "navigation");
    expect(navTool).toBeDefined();
    expect(navTool!.description).toContain("3 items");
  });

  it("deduplicates tools by name", () => {
    const html = `
      <button aria-label="Action">Do</button>
      <button aria-label="Action">Do</button>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const names = tools.map((t) => t.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("returns empty array for plain text HTML", () => {
    const tools = extractToolsFromHTML("<p>Just text</p>", "https://x.com");
    expect(tools).toHaveLength(0);
  });

  it("infers number type from number inputs", () => {
    const html = `
      <form name="settings">
        <input name="count" type="number"/>
        <input name="active" type="checkbox"/>
      </form>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const formTool = tools.find((t) => t.category === "form");
    expect(formTool!.inputSchema.properties["count"]!.type).toBe("number");
    expect(formTool!.inputSchema.properties["active"]!.type).toBe("boolean");
  });
});

describe("extractInternalLinks", () => {
  const base = "https://example.com";

  it("extracts same-origin links", () => {
    const html = `<a href="/about">About</a><a href="https://example.com/docs">Docs</a>`;
    const links = extractInternalLinks(html, base);
    expect(links).toContain("https://example.com/about");
    expect(links).toContain("https://example.com/docs");
  });

  it("ignores external links", () => {
    const html = `<a href="https://other.com/page">External</a>`;
    const links = extractInternalLinks(html, base);
    expect(links).toHaveLength(0);
  });

  it("ignores fragment-only links", () => {
    const html = `<a href="#section">Jump</a>`;
    const links = extractInternalLinks(html, base);
    expect(links).toHaveLength(0);
  });

  it("ignores javascript: links", () => {
    const html = `<a href="javascript:void(0)">Click</a>`;
    const links = extractInternalLinks(html, base);
    expect(links).toHaveLength(0);
  });

  it("ignores mailto: links", () => {
    const html = `<a href="mailto:test@example.com">Email</a>`;
    const links = extractInternalLinks(html, base);
    expect(links).toHaveLength(0);
  });

  it("deduplicates URLs", () => {
    const html = `
      <a href="/page">Page</a>
      <a href="/page">Page again</a>
      <a href="/page#section">Page with hash</a>
    `;
    const links = extractInternalLinks(html, base);
    // Hash is stripped, so all 3 resolve to same URL
    expect(links).toHaveLength(1);
  });

  it("strips hash from URLs", () => {
    const html = `<a href="/page#section">Link</a>`;
    const links = extractInternalLinks(html, base);
    expect(links[0]).toBe("https://example.com/page");
  });
});
