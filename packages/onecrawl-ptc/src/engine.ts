/**
 * PTC Engine — orchestrates script generation, sandboxed execution,
 * and self-healing retry loop.
 */

import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";
import type { RunOptions, EngineRunResult, GenerateOptions } from "./types.js";
import { ToolRegistry, buildToolRegistry } from "./tool-registry.js";
import { createToolBridge } from "./tool-bridge.js";
import { executeSandboxed } from "./sandbox.js";
import { generateScript, type LlmGenerateFn } from "./script-generator.js";

export interface EngineConfig {
  client: OneCrawlClient;
  llmGenerate: LlmGenerateFn;
  registry?: ToolRegistry;
}

export class PtcEngine {
  private readonly client: OneCrawlClient;
  private readonly llmGenerate: LlmGenerateFn;
  private readonly registry: ToolRegistry;

  constructor(config: EngineConfig) {
    this.client = config.client;
    this.llmGenerate = config.llmGenerate;
    this.registry = config.registry ?? buildToolRegistry();
  }

  /**
   * Run a pre-written script with self-healing retry.
   */
  async run(script: string, opts: RunOptions = {}): Promise<EngineRunResult> {
    const maxAttempts = opts.maxAttempts ?? 3;
    const session = opts.session ?? "default";
    const timeout = opts.timeout ?? 120_000;
    const bridge = createToolBridge(this.client, session, this.registry);
    const errors: string[] = [];
    let lastScript = script;

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      const sandboxResult = await executeSandboxed(
        lastScript,
        {
          tools: bridge,
          state: {},
          log: (...args: unknown[]) => {
            // eslint-disable-next-line no-console
            console.log(`[PTC:${attempt}]`, ...args);
          },
        },
        timeout,
      );

      if (sandboxResult.success) {
        return {
          success: true,
          attempts: attempt,
          result: sandboxResult.result,
          errors,
          lastScript,
        };
      }

      const error = sandboxResult.error ?? "Unknown error";
      errors.push(`Attempt ${attempt}: ${error}`);

      if (attempt < maxAttempts) {
        // Self-heal: ask the LLM to fix the script
        const errorContext = `Error: ${error}\nScript that failed:\n${lastScript}`;
        const fixedScript = await generateScript(
          { task: "Fix the following script", provider: "claude" },
          this.registry,
          this.llmGenerate,
          errorContext,
        );
        lastScript = fixedScript;
      }
    }

    return {
      success: false,
      attempts: maxAttempts,
      result: null,
      errors,
      lastScript,
    };
  }

  /**
   * Generate a script from a natural language task, then run it.
   */
  async generateAndRun(
    generateOpts: GenerateOptions,
    runOpts: RunOptions = {},
  ): Promise<EngineRunResult> {
    const script = await generateScript(
      generateOpts,
      this.registry,
      this.llmGenerate,
    );
    return this.run(script, runOpts);
  }

  /**
   * Generate a script without executing it.
   */
  async generate(generateOpts: GenerateOptions): Promise<string> {
    return generateScript(generateOpts, this.registry, this.llmGenerate);
  }
}
