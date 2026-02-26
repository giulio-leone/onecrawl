import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserEvaluateTool = {
  name: "browser_evaluate" as const,
  description: "Execute JavaScript code in the browser page context and return the result.",
  inputSchema: z.object({
    script: z.string().describe("JavaScript code to execute in the browser"),
  }),
  handler: async (
    args: { script: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("evaluate", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
