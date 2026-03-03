/**
 * Sandbox — safe script execution using Node vm module.
 *
 * Scripts are CommonJS-style: they receive `module` and `exports` and must set
 * `module.exports` to an async function `(ctx) => result`.
 *
 * The context `ctx` provides: { tools, state, log }.
 */

import { createContext, Script } from "node:vm";
import type { ToolBridge } from "./tool-bridge.js";

export interface SandboxContext {
  tools: ToolBridge;
  state: Record<string, unknown>;
  log: (...args: unknown[]) => void;
}

export interface SandboxResult {
  success: boolean;
  result: unknown;
  error?: string;
  logs: string[];
}

export async function executeSandboxed(
  scriptText: string,
  ctx: SandboxContext,
  timeoutMs = 120_000,
): Promise<SandboxResult> {
  const logs: string[] = [];

  const logFn = (...args: unknown[]) => {
    const line = args.map((a) => (typeof a === "string" ? a : JSON.stringify(a))).join(" ");
    logs.push(line);
    ctx.log(line);
  };

  // Wrapper that provides a require-like setup for the script
  const wrapper = `
(async function __ptc_main__(__ctx) {
  const module = { exports: {} };
  const exports = module.exports;
  const console = { log: __ctx.__log, warn: __ctx.__log, error: __ctx.__log, info: __ctx.__log };

  // --- user script start ---
  ${scriptText}
  // --- user script end ---

  const fn = typeof module.exports === 'function' ? module.exports : module.exports.default;
  if (typeof fn !== 'function') {
    throw new Error('Script must export an async function via module.exports');
  }
  return fn({ tools: __ctx.tools, state: __ctx.state, log: __ctx.__log });
})
`;

  try {
    const script = new Script(wrapper, {
      filename: "ptc-script.js",
    });

    const sandbox = createContext({
      setTimeout,
      clearTimeout,
      setInterval,
      clearInterval,
      Promise,
      JSON,
      Array,
      Object,
      String,
      Number,
      Boolean,
      Date,
      Math,
      RegExp,
      Error,
      TypeError,
      RangeError,
      Map,
      Set,
      Buffer,
    });

    const runFn = script.runInContext(sandbox, { timeout: timeoutMs });

    const result = await runFn({
      tools: ctx.tools,
      state: ctx.state,
      __log: logFn,
    });

    return { success: true, result, logs };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, result: null, error: message, logs };
  }
}
