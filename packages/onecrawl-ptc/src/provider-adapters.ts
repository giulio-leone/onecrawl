/**
 * Provider Adapters — format PTC prompts for different LLM providers.
 */

import type { ProviderAdapter } from "./types.js";

const claudeAdapter: ProviderAdapter = {
  name: "claude",
  formatPrompt(task: string, toolDocs: string, errorContext?: string): string {
    const errorBlock = errorContext
      ? `\n\nThe previous attempt failed with this error:\n\`\`\`\n${errorContext}\n\`\`\`\nFix the issue and try again.`
      : "";
    return `You are a browser automation assistant. Write a Node.js script that uses the OneCrawl tool bridge.

## Available Tools
${toolDocs}

## Task
${task}${errorBlock}

## Script Format
The script must:
1. Export an async function via \`module.exports\`
2. The function receives \`{ tools, state, log }\`
3. Call tools via \`await tools.call("name", { param: value })\`
4. Return a result object summarising what was done
5. Use try/catch for error handling
6. Use \`log()\` for status messages

Respond with ONLY the JavaScript code, no markdown fences.`;
  },
};

const openaiAdapter: ProviderAdapter = {
  name: "openai",
  formatPrompt(task: string, toolDocs: string, errorContext?: string): string {
    const errorBlock = errorContext
      ? `\n\nPrevious error:\n${errorContext}\nFix it.`
      : "";
    return `Write a Node.js automation script for the following task.

Tools available (call via tools.call("name", {params})):
${toolDocs}

Task: ${task}${errorBlock}

Export an async function: module.exports = async ({ tools, state, log }) => { ... }
Return a result object. Use try/catch. Output only code.`;
  },
};

const geminiAdapter: ProviderAdapter = {
  name: "gemini",
  formatPrompt(task: string, toolDocs: string, errorContext?: string): string {
    const errorBlock = errorContext
      ? `\nPrevious attempt failed: ${errorContext}\n`
      : "";
    return `Generate a Node.js script for browser automation.

Available tools (invoked via tools.call("toolName", {params})):
${toolDocs}

Task: ${task}${errorBlock}

The script must export module.exports = async ({ tools, state, log }) => { return result; }
Only output JavaScript code.`;
  },
};

const adapters: Record<string, ProviderAdapter> = {
  claude: claudeAdapter,
  openai: openaiAdapter,
  gemini: geminiAdapter,
};

export function getAdapter(provider: string): ProviderAdapter {
  const adapter = adapters[provider];
  if (!adapter) {
    throw new Error(`Unknown provider: ${provider}. Available: ${Object.keys(adapters).join(", ")}`);
  }
  return adapter;
}

export { adapters };
