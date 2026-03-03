/**
 * Script Generator — produces PTC scripts from natural language tasks.
 *
 * Uses provider adapters to format prompts. The actual LLM call is
 * delegated to a pluggable `generate` function so the engine is
 * truly model-agnostic and testable without API keys.
 */

import type { GenerateOptions } from "./types.js";
import { getAdapter } from "./provider-adapters.js";
import type { ToolRegistry } from "./tool-registry.js";

export type LlmGenerateFn = (prompt: string) => Promise<string>;

export function buildPrompt(
  task: string,
  registry: ToolRegistry,
  provider: string = "claude",
  errorContext?: string,
): string {
  const adapter = getAdapter(provider);
  const toolDocs = registry.toDocString();
  return adapter.formatPrompt(task, toolDocs, errorContext);
}

export async function generateScript(
  opts: GenerateOptions,
  registry: ToolRegistry,
  llmGenerate: LlmGenerateFn,
  errorContext?: string,
): Promise<string> {
  const prompt = buildPrompt(opts.task, registry, opts.provider, errorContext);
  const raw = await llmGenerate(prompt);
  return stripCodeFences(raw);
}

/** Remove markdown code fences if the model wraps its response. */
function stripCodeFences(code: string): string {
  let cleaned = code.trim();
  // Remove ```javascript ... ``` or ```js ... ``` or ``` ... ```
  const fenceRe = /^```(?:javascript|js)?\s*\n?([\s\S]*?)```\s*$/;
  const match = fenceRe.exec(cleaned);
  if (match) {
    cleaned = match[1].trim();
  }
  return cleaned;
}
