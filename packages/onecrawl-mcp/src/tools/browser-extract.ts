import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserExtractTool = {
  name: "browser_extract" as const,
  description: "Extract text content or attributes from elements on the page.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the element(s) to extract from"),
    attribute: z
      .string()
      .optional()
      .describe("HTML attribute to extract (omit for text content)"),
    multiple: z
      .boolean()
      .optional()
      .describe("Extract from all matching elements (default: false)"),
  }),
  handler: async (
    args: { selector: string; attribute?: string; multiple?: boolean },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("extract", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
