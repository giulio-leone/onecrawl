import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserNavigateTool = {
  name: "browser_navigate" as const,
  description: "Navigate to a URL in the browser. Supports wait conditions.",
  inputSchema: z.object({
    url: z.string().url().describe("URL to navigate to"),
    waitUntil: z
      .enum(["load", "domcontentloaded", "networkidle"])
      .optional()
      .describe("When to consider navigation complete"),
  }),
  handler: async (
    args: { url: string; waitUntil?: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("navigate", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
