import { describe, it, expect, vi } from "vitest";
import { ToolRegistry, buildToolRegistry } from "../src/tool-registry.js";
import { serializeArgs } from "../src/tool-bridge.js";
import { executeSandboxed } from "../src/sandbox.js";
import { buildPrompt } from "../src/script-generator.js";
import { getAdapter } from "../src/provider-adapters.js";
import type { ToolSpec } from "../src/types.js";

describe("ToolRegistry", () => {
  it("loads default tools", () => {
    const reg = buildToolRegistry();
    expect(reg.list().length).toBeGreaterThan(20);
    expect(reg.has("navigate")).toBe(true);
    expect(reg.has("click")).toBe(true);
  });

  it("get returns spec or undefined", () => {
    const reg = buildToolRegistry();
    expect(reg.get("navigate")?.name).toBe("navigate");
    expect(reg.get("nonexistent")).toBeUndefined();
  });

  it("toDocString includes tool names", () => {
    const reg = buildToolRegistry();
    const doc = reg.toDocString();
    expect(doc).toContain("navigate");
    expect(doc).toContain("click");
  });

  it("accepts extra tools", () => {
    const extra: ToolSpec = { name: "custom-tool", description: "A custom tool", usage: "custom-tool", positionalArgs: [] };
    const reg = buildToolRegistry([extra]);
    expect(reg.has("custom-tool")).toBe(true);
    expect(reg.list().length).toBeGreaterThan(20);
  });
});

describe("serializeArgs", () => {
  it("maps positional args in order", () => {
    const spec: ToolSpec = { name: "viewport", description: "", usage: "", positionalArgs: ["width", "height"] };
    const result = serializeArgs(spec, { width: 1920, height: 1080 });
    expect(result).toEqual(["1920", "1080"]);
  });

  it("adds flags for non-positional params", () => {
    const spec: ToolSpec = { name: "requests", description: "", usage: "", positionalArgs: [] };
    const result = serializeArgs(spec, { filter: "*.json", verbose: true });
    expect(result).toContain("--filter=*.json");
    expect(result).toContain("--verbose");
  });

  it("ignores undefined/null/false params", () => {
    const spec: ToolSpec = { name: "click", description: "", usage: "", positionalArgs: ["target"] };
    const result = serializeArgs(spec, { target: "#btn", optional: undefined, disabled: false });
    expect(result).toEqual(["#btn"]);
  });
});

describe("executeSandboxed", () => {
  it("runs a simple script returning a value", async () => {
    const script = `module.exports = async ({ log }) => { log("hello"); return 42; };`;
    const logs: string[] = [];
    const result = await executeSandboxed(script, {
      tools: { call: vi.fn(), listTools: () => [] },
      state: {},
      log: (...args: unknown[]) => logs.push(args.join(" ")),
    });
    expect(result.success).toBe(true);
    expect(result.result).toBe(42);
    expect(result.logs).toContain("hello");
  });

  it("catches script errors", async () => {
    const script = `module.exports = async () => { throw new Error("boom"); };`;
    const result = await executeSandboxed(script, {
      tools: { call: vi.fn(), listTools: () => [] },
      state: {},
      log: () => {},
    });
    expect(result.success).toBe(false);
    expect(result.error).toContain("boom");
  });

  it("fails if script does not export a function", async () => {
    const script = `module.exports = "not a function";`;
    const result = await executeSandboxed(script, {
      tools: { call: vi.fn(), listTools: () => [] },
      state: {},
      log: () => {},
    });
    expect(result.success).toBe(false);
    expect(result.error).toContain("must export an async function");
  });

  it("provides tools.call to the script", async () => {
    const mockCall = vi.fn().mockResolvedValue({ exitCode: 0, stdout: "ok", stderr: "" });
    const script = `module.exports = async ({ tools }) => {
      const r = await tools.call("navigate", { url: "https://example.com" });
      return r.stdout;
    };`;
    const result = await executeSandboxed(script, {
      tools: { call: mockCall, listTools: () => ["navigate"] },
      state: {},
      log: () => {},
    });
    expect(result.success).toBe(true);
    expect(result.result).toBe("ok");
    expect(mockCall).toHaveBeenCalledWith("navigate", { url: "https://example.com" });
  });
});

describe("buildPrompt", () => {
  it("builds a prompt for claude", () => {
    const reg = buildToolRegistry();
    const prompt = buildPrompt("Search for jobs", reg, "claude");
    expect(prompt).toContain("Search for jobs");
    expect(prompt).toContain("navigate");
    expect(prompt).toContain("module.exports");
  });

  it("includes error context when provided", () => {
    const reg = buildToolRegistry();
    const prompt = buildPrompt("Fix it", reg, "claude", "TypeError: x is not defined");
    expect(prompt).toContain("TypeError: x is not defined");
  });
});

describe("ProviderAdapters", () => {
  it("has claude, openai, gemini adapters", () => {
    expect(getAdapter("claude").name).toBe("claude");
    expect(getAdapter("openai").name).toBe("openai");
    expect(getAdapter("gemini").name).toBe("gemini");
  });

  it("throws for unknown provider", () => {
    expect(() => getAdapter("unknown")).toThrow("Unknown provider");
  });
});
