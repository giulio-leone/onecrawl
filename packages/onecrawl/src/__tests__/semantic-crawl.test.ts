import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  globToRegex,
  matchesPatterns,
  extractToolsFromHTML,
  extractInternalLinks,
} from "../utils/semantic-extractor.js";
import { SemanticCrawlUseCase } from "../use-cases/semantic-crawl.use-case.js";
import type { ScraperPort, ScrapeResponse } from "../ports/index.js";
import type { CrawlTarget } from "../domain/semantic-tool.js";

// ---------------------------------------------------------------------------
// globToRegex
// ---------------------------------------------------------------------------

describe("globToRegex", () => {
  it("matches exact strings", () => {
    expect(globToRegex("hello").test("hello")).toBe(true);
    expect(globToRegex("hello").test("world")).toBe(false);
  });

  it("handles single wildcard *", () => {
    const re = globToRegex("*.html");
    expect(re.test("index.html")).toBe(true);
    expect(re.test("dir/index.html")).toBe(false); // * does not match /
  });

  it("handles double wildcard **", () => {
    const re = globToRegex("https://example.com/**");
    expect(re.test("https://example.com/a/b/c")).toBe(true);
  });

  it("handles question mark ?", () => {
    const re = globToRegex("file?.txt");
    expect(re.test("file1.txt")).toBe(true);
    expect(re.test("file12.txt")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// matchesPatterns
// ---------------------------------------------------------------------------

describe("matchesPatterns", () => {
  it("returns true when any pattern matches", () => {
    expect(
      matchesPatterns("https://example.com/docs/api", [
        "https://example.com/docs/**",
      ]),
    ).toBe(true);
  });

  it("returns false when no pattern matches", () => {
    expect(
      matchesPatterns("https://other.com/page", [
        "https://example.com/**",
      ]),
    ).toBe(false);
  });

  it("handles multiple patterns", () => {
    expect(
      matchesPatterns("https://example.com/blog/post", [
        "https://example.com/docs/**",
        "https://example.com/blog/**",
      ]),
    ).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// extractToolsFromHTML
// ---------------------------------------------------------------------------

describe("extractToolsFromHTML", () => {
  it("extracts form tools with inputs", () => {
    const html = `
      <form name="login">
        <input name="username" type="text" placeholder="Username" required />
        <input name="password" type="password" placeholder="Password" required />
        <button type="submit">Log in</button>
      </form>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    expect(tools.length).toBe(1);
    expect(tools[0].name).toBe("form_login");
    expect(tools[0].category).toBe("form");
    expect(tools[0].inputSchema.properties).toHaveProperty("username");
    expect(tools[0].inputSchema.properties).toHaveProperty("password");
    expect(tools[0].inputSchema.required).toEqual(["username", "password"]);
  });

  it("extracts form with aria-label", () => {
    const html = `
      <form aria-label="Contact Us">
        <input name="email" type="email" />
        <textarea name="message" placeholder="Your message"></textarea>
      </form>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    expect(tools.length).toBe(1);
    expect(tools[0].name).toBe("form_contact_us");
    expect(tools[0].inputSchema.properties).toHaveProperty("email");
    expect(tools[0].inputSchema.properties).toHaveProperty("message");
  });

  it("extracts search inputs", () => {
    const html = `<input type="search" name="q" aria-label="Site Search" placeholder="Search…" />`;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const search = tools.find((t) => t.category === "search");
    expect(search).toBeDefined();
    expect(search!.name).toContain("search_");
    expect(search!.inputSchema.required).toContain("q");
  });

  it("extracts buttons with aria-label", () => {
    const html = `<button aria-label="Open menu">☰</button>`;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const btn = tools.find((t) => t.category === "button");
    expect(btn).toBeDefined();
    expect(btn!.name).toBe("button_open_menu");
  });

  it("extracts buttons with text content", () => {
    const html = `<button>Download PDF</button>`;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const btn = tools.find((t) => t.category === "button");
    expect(btn).toBeDefined();
    expect(btn!.name).toBe("button_download_pdf");
  });

  it("skips submit and reset buttons", () => {
    const html = `
      <button type="submit">Submit</button>
      <button type="reset">Reset</button>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    expect(tools.filter((t) => t.category === "button")).toHaveLength(0);
  });

  it("extracts nav tools", () => {
    const html = `
      <nav aria-label="Main Menu">
        <a href="/home">Home</a>
        <a href="/about">About</a>
        <a href="/contact">Contact</a>
      </nav>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    const nav = tools.find((t) => t.category === "navigation");
    expect(nav).toBeDefined();
    expect(nav!.name).toBe("nav_main_menu");
    expect(nav!.inputSchema.properties.item).toBeDefined();
    expect((nav!.inputSchema.properties.item as { description?: string }).description).toContain("Home");
  });

  it("deduplicates tools by name", () => {
    const html = `
      <button aria-label="Toggle">On</button>
      <button aria-label="Toggle">Off</button>
    `;
    const tools = extractToolsFromHTML(html, "https://example.com");
    expect(tools.filter((t) => t.name === "button_toggle")).toHaveLength(1);
  });

  it("returns empty array for html with no interactive elements", () => {
    const html = `<p>Hello world</p><div>Static content</div>`;
    const tools = extractToolsFromHTML(html, "https://example.com");
    expect(tools).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// extractInternalLinks
// ---------------------------------------------------------------------------

describe("extractInternalLinks", () => {
  it("extracts same-origin links", () => {
    const html = `<a href="/about">About</a><a href="/contact">Contact</a>`;
    const links = extractInternalLinks(html, "https://example.com/");
    expect(links).toContain("https://example.com/about");
    expect(links).toContain("https://example.com/contact");
  });

  it("excludes external links", () => {
    const html = `<a href="https://other.com/page">External</a>`;
    const links = extractInternalLinks(html, "https://example.com/");
    expect(links).toHaveLength(0);
  });

  it("strips hash fragments", () => {
    const html = `<a href="/page#section">Section</a>`;
    const links = extractInternalLinks(html, "https://example.com/");
    expect(links).toContain("https://example.com/page");
    expect(links.some((l) => l.includes("#"))).toBe(false);
  });

  it("deduplicates links", () => {
    const html = `
      <a href="/page">1</a>
      <a href="/page">2</a>
      <a href="/page#foo">3</a>
    `;
    const links = extractInternalLinks(html, "https://example.com/");
    expect(links.filter((l) => l === "https://example.com/page")).toHaveLength(1);
  });

  it("skips javascript: and mailto: hrefs", () => {
    const html = `
      <a href="javascript:void(0)">JS</a>
      <a href="mailto:a@b.com">Mail</a>
    `;
    const links = extractInternalLinks(html, "https://example.com/");
    expect(links).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// SemanticCrawlUseCase
// ---------------------------------------------------------------------------

describe("SemanticCrawlUseCase", () => {
  const makeResponse = (html: string, url: string): ScrapeResponse => ({
    result: {
      url,
      title: "Test",
      content: "",
      html,
    },
    cached: false,
    duration: 10,
    source: "test",
  });

  let mockScraper: ScraperPort;

  beforeEach(() => {
    mockScraper = {
      scrape: vi.fn(),
      scrapeMany: vi.fn(),
      isAvailable: vi.fn().mockResolvedValue(true),
      getName: () => "mock",
    };
  });

  it("discovers tools across multiple pages", async () => {
    const pages: Record<string, string> = {
      "https://example.com": `
        <a href="/login">Login</a>
        <nav aria-label="Main"><a href="/login">Login</a></nav>
      `,
      "https://example.com/login": `
        <form name="loginForm">
          <input name="user" type="text" />
          <input name="pass" type="password" />
        </form>
      `,
    };

    vi.mocked(mockScraper.scrape).mockImplementation(async (url) => {
      const html = pages[url] ?? "<p>empty</p>";
      return makeResponse(html, url);
    });

    const uc = new SemanticCrawlUseCase(mockScraper);
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com"],
      maxPages: 10,
      maxDepth: 2,
    };

    const result = await uc.execute(target);
    expect(result.pagesScanned).toBeGreaterThanOrEqual(1);
    expect(result.toolsDiscovered).toBeGreaterThanOrEqual(1);
    expect(result.errors).toHaveLength(0);
  });

  it("respects maxPages limit", async () => {
    vi.mocked(mockScraper.scrape).mockImplementation(async (url) =>
      makeResponse(
        `<a href="/page-${Date.now()}">next</a>
         <form name="f"><input name="x" /></form>`,
        url,
      ),
    );

    const uc = new SemanticCrawlUseCase(mockScraper);
    const result = await uc.execute({
      site: "example.com",
      entryPoints: ["https://example.com"],
      maxPages: 3,
      maxDepth: 10,
    });

    expect(result.pagesScanned).toBeLessThanOrEqual(3);
  });

  it("records errors without crashing", async () => {
    vi.mocked(mockScraper.scrape).mockRejectedValue(new Error("timeout"));

    const uc = new SemanticCrawlUseCase(mockScraper);
    const result = await uc.execute({
      site: "example.com",
      entryPoints: ["https://example.com"],
      maxPages: 5,
      maxDepth: 1,
    });

    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0]).toContain("timeout");
  });

  it("can be cancelled", async () => {
    vi.mocked(mockScraper.scrape).mockImplementation(
      (_url, opts) =>
        new Promise((_resolve, reject) => {
          const timer = setTimeout(() => {}, 10_000);
          opts?.signal?.addEventListener("abort", () => {
            clearTimeout(timer);
            reject(new Error("aborted"));
          });
        }),
    );

    const uc = new SemanticCrawlUseCase(mockScraper);
    const promise = uc.execute({
      site: "example.com",
      entryPoints: ["https://example.com"],
      maxPages: 100,
      maxDepth: 5,
    });

    expect(uc.isRunning()).toBe(true);
    // Allow the execute loop to reach the await
    await new Promise((r) => setTimeout(r, 10));
    uc.cancel();
    const result = await promise;
    expect(uc.isRunning()).toBe(false);
    expect(result.pagesScanned).toBeLessThanOrEqual(1);
  });

  it("calls onProgress callback", async () => {
    vi.mocked(mockScraper.scrape).mockImplementation(async (url) =>
      makeResponse("<p>hello</p>", url),
    );

    const progressCb = vi.fn();
    const uc = new SemanticCrawlUseCase(mockScraper);
    await uc.execute(
      {
        site: "example.com",
        entryPoints: ["https://example.com"],
        maxPages: 1,
        maxDepth: 0,
      },
      progressCb,
    );

    expect(progressCb).toHaveBeenCalled();
  });
});
