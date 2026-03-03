/**
 * Tool Bridge — exposes OneCrawl CLI tools as callable async functions
 * inside the PTC sandbox.
 */

import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";
import type { ToolSpec, ToolCallResult } from "./types.js";
import { ToolRegistry } from "./tool-registry.js";

/** Serialise tool params into CLI args array following positional-first convention. */
export function serializeArgs(
  spec: ToolSpec,
  params: Record<string, unknown>,
): string[] {
  const args: string[] = [];
  const consumed = new Set<string>();

  // Positional args first (in order declared by the spec)
  for (const key of spec.positionalArgs) {
    if (params[key] !== undefined && params[key] !== null) {
      args.push(String(params[key]));
      consumed.add(key);
    }
  }

  // Remaining params as --key=value flags
  for (const [k, v] of Object.entries(params)) {
    if (consumed.has(k)) continue;
    if (v === true) {
      args.push(`--${k}`);
    } else if (v !== undefined && v !== null && v !== false) {
      args.push(`--${k}=${String(v)}`);
    }
  }

  return args;
}

export interface ToolBridge {
  /** Call a tool by name with params object. */
  call(name: string, params?: Record<string, unknown>): Promise<ToolCallResult>;
  /** List available tool names. */
  listTools(): string[];
}

export function createToolBridge(
  client: OneCrawlClient,
  session: string,
  registry: ToolRegistry,
): ToolBridge {
  return {
    async call(name: string, params: Record<string, unknown> = {}): Promise<ToolCallResult> {
      const spec = registry.get(name);
      if (!spec) {
        return { exitCode: 1, stdout: "", stderr: `Unknown tool: ${name}` };
      }
      const args = serializeArgs(spec, params);
      try {
        const result = await client.cli.execute(name, args, { session });
        return {
          exitCode: result.exitCode,
          stdout: result.stdout ?? "",
          stderr: result.stderr ?? "",
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { exitCode: 1, stdout: "", stderr: msg };
      }
    },
    listTools(): string[] {
      return registry.list().map((t) => t.name);
    },
  };
}
