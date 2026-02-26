import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserWaitTool = {
  name: "browser_wait" as const,
  description: "Wait for an element to appear, a URL to match, or a timeout to elapse.",
  inputSchema: z.object({
    selector: z
      .string()
      .optional()
      .describe("CSS selector to wait for"),
    url: z
      .string()
      .optional()
      .describe("URL pattern to wait for (substring match)"),
    timeout: z
      .number()
      .optional()
      .describe("Maximum wait time in milliseconds (default: 30000)"),
  }),
  handler: async (
    args: { selector?: string; url?: string; timeout?: number },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("wait", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
