import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserTypeTool = {
  name: "browser_type" as const,
  description: "Type text into an input element on the page.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the input element"),
    text: z.string().describe("Text to type"),
    clearFirst: z
      .boolean()
      .optional()
      .describe("Clear the input before typing (default: false)"),
  }),
  handler: async (
    args: { selector: string; text: string; clearFirst?: boolean },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("type", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
