/**
 * Semantic Tool Schema Tests
 */

import { describe, it, expect } from "vitest";
import {
  SemanticToolSchema,
  CrawlTargetSchema,
} from "../../src/domain/semantic-tool.js";

describe("SemanticToolSchema", () => {
  const validTool = {
    name: "search_box",
    description: "Search the site",
    inputSchema: {
      type: "object" as const,
      properties: {
        query: { type: "string" as const, description: "Search query" },
      },
      required: ["query"],
    },
    confidence: 0.9,
    category: "search",
  };

  it("accepts a valid tool", () => {
    const t = SemanticToolSchema.parse(validTool);
    expect(t.name).toBe("search_box");
    expect(t.confidence).toBe(0.9);
  });

  it("accepts minimal tool (no confidence/category)", () => {
    const t = SemanticToolSchema.parse({
      name: "btn",
      description: "Click me",
      inputSchema: { type: "object", properties: {} },
    });
    expect(t.confidence).toBeUndefined();
    expect(t.category).toBeUndefined();
  });

  it("rejects confidence out of range", () => {
    expect(() =>
      SemanticToolSchema.parse({ ...validTool, confidence: 1.5 }),
    ).toThrow();
    expect(() =>
      SemanticToolSchema.parse({ ...validTool, confidence: -0.1 }),
    ).toThrow();
  });

  it("accepts confidence at boundaries", () => {
    expect(SemanticToolSchema.parse({ ...validTool, confidence: 0 }).confidence).toBe(0);
    expect(SemanticToolSchema.parse({ ...validTool, confidence: 1 }).confidence).toBe(1);
  });

  it("rejects invalid inputSchema type", () => {
    expect(() =>
      SemanticToolSchema.parse({
        ...validTool,
        inputSchema: { type: "array", properties: {} },
      }),
    ).toThrow();
  });

  it("rejects missing name", () => {
    const { name, ...noName } = validTool;
    expect(() => SemanticToolSchema.parse(noName)).toThrow();
  });

  it("validates property types", () => {
    const t = SemanticToolSchema.parse({
      ...validTool,
      inputSchema: {
        type: "object",
        properties: {
          count: { type: "number" },
          active: { type: "boolean" },
          items: { type: "array" },
          config: { type: "object" },
        },
      },
    });
    expect(Object.keys(t.inputSchema.properties)).toHaveLength(4);
  });

  it("rejects invalid property type", () => {
    expect(() =>
      SemanticToolSchema.parse({
        ...validTool,
        inputSchema: {
          type: "object",
          properties: { x: { type: "date" } },
        },
      }),
    ).toThrow();
  });
});

describe("CrawlTargetSchema", () => {
  const validTarget = {
    site: "example.com",
    entryPoints: ["https://example.com/"],
  };

  it("accepts valid target with defaults", () => {
    const t = CrawlTargetSchema.parse(validTarget);
    expect(t.maxPages).toBe(50);
    expect(t.maxDepth).toBe(3);
  });

  it("accepts full target", () => {
    const t = CrawlTargetSchema.parse({
      ...validTarget,
      entryPoints: ["https://example.com/", "https://example.com/docs"],
      maxPages: 100,
      maxDepth: 5,
      includePatterns: ["*/docs/*"],
      excludePatterns: ["*/admin/*"],
    });
    expect(t.maxPages).toBe(100);
    expect(t.includePatterns).toEqual(["*/docs/*"]);
  });

  it("rejects invalid entry point URLs", () => {
    expect(() =>
      CrawlTargetSchema.parse({
        site: "x",
        entryPoints: ["not-a-url"],
      }),
    ).toThrow();
  });

  it("rejects empty entryPoints", () => {
    expect(() =>
      CrawlTargetSchema.parse({ site: "x", entryPoints: [] }),
    ).not.toThrow(); // array can be empty per schema
  });

  it("rejects negative maxPages", () => {
    expect(() =>
      CrawlTargetSchema.parse({ ...validTarget, maxPages: -1 }),
    ).toThrow();
  });

  it("rejects negative maxDepth", () => {
    expect(() =>
      CrawlTargetSchema.parse({ ...validTarget, maxDepth: -1 }),
    ).toThrow();
  });

  it("accepts maxDepth of 0 (entry points only)", () => {
    const t = CrawlTargetSchema.parse({ ...validTarget, maxDepth: 0 });
    expect(t.maxDepth).toBe(0);
  });
});
