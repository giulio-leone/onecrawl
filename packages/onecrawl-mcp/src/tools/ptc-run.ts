/**
 * MCP Tool: ptc_run — execute a PTC automation script.
 */

import { z } from "zod";
import type { McpToolDef } from "./generated-cli-tools.js";

export const ptcRunTool: McpToolDef = {
  name: "ptc_run",
  description:
    "Execute a Programmatic Tool Calling script. The script calls OneCrawl tools " +
    "as functions with self-healing retry on failure.",
  inputSchema: z.object({
    script: z.string().describe("JavaScript script content to execute"),
    session: z
      .string()
      .optional()
      .default("default")
      .describe("Browser session name"),
    maxAttempts: z
      .number()
      .optional()
      .default(3)
      .describe("Maximum self-healing retry attempts"),
  }),
  handler: async (params, client) => {
    const result = await client.webAction("cli-exec", {
      command: "ptc",
      args: [
        "run",
        "--inline",
        `--session=${(params.session as string) ?? "default"}`,
        `--max-attempts=${(params.maxAttempts as number) ?? 3}`,
      ],
      stdin: params.script as string,
    });
    return result;
  },
};
