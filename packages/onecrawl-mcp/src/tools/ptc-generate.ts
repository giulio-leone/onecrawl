/**
 * MCP Tool: ptc_generate — generate a PTC automation script from a task.
 */

import { z } from "zod";
import type { McpToolDef } from "./generated-cli-tools.js";

export const ptcGenerateTool: McpToolDef = {
  name: "ptc_generate",
  description:
    "Generate a Programmatic Tool Calling script from a natural language task description. " +
    "Returns executable JavaScript that calls OneCrawl tools as functions.",
  inputSchema: z.object({
    task: z.string().describe("Natural language description of the automation task"),
    provider: z
      .enum(["claude", "openai", "gemini"])
      .optional()
      .default("claude")
      .describe("LLM provider for prompt formatting"),
    context: z.string().optional().describe("Additional context for the generation"),
  }),
  handler: async (params, client) => {
    const result = await client.webAction("cli-exec", {
      command: "ptc",
      args: ["generate", params.task as string, `--provider=${(params.provider as string) ?? "claude"}`],
    });
    return result;
  },
};
