/**
 * React Native Compatibility Tests
 *
 * Verifies that the OneCrawl native entry point does not depend on
 * Node-specific built-in modules and exports the correct symbols.
 */
import { describe, it, expect } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const NODE_BUILTINS = [
  "fs",
  "path",
  "os",
  "child_process",
  "net",
  "http",
  "https",
  "stream",
  "crypto",
  "buffer",
];

const IMPORT_PATTERNS = NODE_BUILTINS.flatMap((mod) => [
  new RegExp(`from\\s+["'](?:node:)?${mod}(?:/[^"']*)?["']`),
  new RegExp(`require\\s*\\(\\s*["'](?:node:)?${mod}(?:/[^"']*)?["']\\s*\\)`),
]);

/** Recursively collect .ts source files, skipping tests / node_modules */
function collectSourceFiles(dir: string): string[] {
  const results: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "__tests__" || entry.name === "node_modules") continue;
      results.push(...collectSourceFiles(full));
    } else if (
      entry.isFile() &&
      entry.name.endsWith(".ts") &&
      !entry.name.endsWith(".test.ts") &&
      entry.name !== "cli.ts"
    ) {
      results.push(full);
    }
  }
  return results;
}

/**
 * Collect source files reachable from `index.native.ts`.
 * Excludes Node-only adapter directories (playwright, cdp, undici)
 * and the Node-only FsStorageAdapter file.
 */
function collectNativeSourceFiles(srcDir: string): string[] {
  const nativeDirs = [
    path.join(srcDir, "domain"),
    path.join(srcDir, "ports"),
    path.join(srcDir, "adapters", "fetch"),
    path.join(srcDir, "adapters", "fetch-pool"),
    path.join(srcDir, "adapters", "search-engines"),
    path.join(srcDir, "adapters", "storage"),
    path.join(srcDir, "adapters", "shared"),
    path.join(srcDir, "use-cases"),
    path.join(srcDir, "auth"),
    path.join(srcDir, "utils"),
  ];

  const nodeOnlyFiles = new Set(["fs-storage.adapter.ts"]);

  const files: string[] = [];
  for (const dir of nativeDirs) {
    if (!fs.existsSync(dir)) continue;
    for (const f of collectSourceFiles(dir)) {
      if (!nodeOnlyFiles.has(path.basename(f))) {
        files.push(f);
      }
    }
  }
  return files;
}

// ---------------------------------------------------------------------------
// 1. Source scanning — no Node.js imports in native-reachable files
// ---------------------------------------------------------------------------

describe("React Native compatibility", () => {
  const srcDir = path.resolve(__dirname, "..");
  const nativeFiles = collectNativeSourceFiles(srcDir);

  describe("no Node-specific imports in native source", () => {
    it("should have source files to check", () => {
      expect(nativeFiles.length).toBeGreaterThan(0);
    });

    it.each(nativeFiles)("file %s has no Node-only imports", (file) => {
      const content = fs.readFileSync(file, "utf-8");
      for (const pat of IMPORT_PATTERNS) {
        expect(content).not.toMatch(pat);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Export validation — native entry exports key cross-platform symbols
  // ---------------------------------------------------------------------------

  describe("native entry exports cross-platform symbols", () => {
    it("exports FetchScraperAdapter", async () => {
      const mod = await import("../index.native.js");
      expect(mod.FetchScraperAdapter).toBeTypeOf("function");
    });

    it("exports FetchPoolScraperAdapter", async () => {
      const mod = await import("../index.native.js");
      expect(mod.FetchPoolScraperAdapter).toBeTypeOf("function");
    });

    it("exports SearchAdapter", async () => {
      const mod = await import("../index.native.js");
      expect(mod.SearchAdapter).toBeTypeOf("function");
    });

    it("exports MemoryStorageAdapter", async () => {
      const mod = await import("../index.native.js");
      expect(mod.MemoryStorageAdapter).toBeTypeOf("function");
    });

    it("ports module is importable (StoragePort is type-only)", async () => {
      const mod = await import("../ports/index.js");
      expect(mod).toBeDefined();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Negative testing — Node-only adapters excluded from native entry
  // ---------------------------------------------------------------------------

  describe("native entry does NOT export Node-only adapters", () => {
    it("does not export PlaywrightScraperAdapter", async () => {
      const mod = await import("../index.native.js");
      const keys = Object.keys(mod);
      expect(keys).not.toContain("PlaywrightScraperAdapter");
      expect(keys).not.toContain("PlaywrightBrowserAdapter");
    });

    it("does not export CDPScraperAdapter", async () => {
      const mod = await import("../index.native.js");
      const keys = Object.keys(mod);
      expect(keys).not.toContain("CDPScraperAdapter");
      expect(keys).not.toContain("CDPClient");
    });

    it("does not export UndiciScraperAdapter", async () => {
      const mod = await import("../index.native.js");
      const keys = Object.keys(mod);
      expect(keys).not.toContain("UndiciScraperAdapter");
    });

    it("does not export FsStorageAdapter", async () => {
      const mod = await import("../index.native.js");
      const keys = Object.keys(mod);
      expect(keys).not.toContain("FsStorageAdapter");
      expect(keys).not.toContain("createFsStorageAdapter");
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Adapter constructibility — cross-platform adapters can be instantiated
  // ---------------------------------------------------------------------------

  describe("cross-platform adapters are constructible", () => {
    it("FetchScraperAdapter can be instantiated", async () => {
      const { FetchScraperAdapter } = await import("../index.native.js");
      const adapter = new FetchScraperAdapter();
      expect(adapter).toBeDefined();
      expect(typeof adapter.scrape).toBe("function");
    });

    it("FetchPoolScraperAdapter can be instantiated", async () => {
      const { FetchPoolScraperAdapter } = await import("../index.native.js");
      const adapter = new FetchPoolScraperAdapter();
      expect(adapter).toBeDefined();
      expect(typeof adapter.scrape).toBe("function");
    });

    it("MemoryStorageAdapter can be instantiated", async () => {
      const { MemoryStorageAdapter } = await import("../index.native.js");
      const adapter = new MemoryStorageAdapter();
      expect(adapter).toBeDefined();
      expect(typeof adapter.get).toBe("function");
      expect(typeof adapter.set).toBe("function");
    });

    it("SearchAdapter can be instantiated with a scraper", async () => {
      const { SearchAdapter, FetchScraperAdapter } = await import(
        "../index.native.js"
      );
      const scraper = new FetchScraperAdapter();
      const adapter = new SearchAdapter(scraper);
      expect(adapter).toBeDefined();
      expect(typeof adapter.search).toBe("function");
    });
  });
});
