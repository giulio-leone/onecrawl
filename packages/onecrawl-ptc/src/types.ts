/**
 * @giulio-leone/onecrawl-ptc — Types
 */

/** Specification for a single CLI tool. */
export interface ToolSpec {
  name: string;
  description: string;
  usage: string;
  /** Positional arguments in order (mapped before --flags). */
  positionalArgs: string[];
  /** Default values for optional flags. */
  defaults?: Record<string, unknown>;
}

/** Result of a single tool invocation via the bridge. */
export interface ToolCallResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

/** Options for running a PTC script. */
export interface RunOptions {
  /** Active Playwright session name. */
  session?: string;
  /** OneCrawl server URL. */
  onecrawlUrl?: string;
  /** Maximum self-healing retry attempts. */
  maxAttempts?: number;
  /** Timeout per script execution in ms. */
  timeout?: number;
}

/** Options for generating a script. */
export interface GenerateOptions {
  /** Provider hint for prompt formatting. */
  provider?: "claude" | "openai" | "gemini";
  /** Task description in natural language. */
  task: string;
  /** Extra context injected into the template. */
  context?: string;
}

/** Result of an engine run (possibly with self-healing). */
export interface EngineRunResult {
  success: boolean;
  attempts: number;
  result: unknown;
  errors: string[];
  lastScript: string;
}

/** Provider adapter interface — formats prompts for a specific LLM. */
export interface ProviderAdapter {
  name: string;
  formatPrompt(task: string, toolDocs: string, errorContext?: string): string;
}
