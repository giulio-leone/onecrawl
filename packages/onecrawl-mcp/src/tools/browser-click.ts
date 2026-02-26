import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserClickTool = {
  name: "browser_click" as const,
  description: "Click an element on the page by CSS selector or visible text.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the element to click"),
    text: z.string().optional().describe("Visible text to match within the selector"),
    button: z
      .enum(["left", "right"])
      .optional()
      .describe("Mouse button to use (default: left)"),
  }),
  handler: async (
    args: { selector: string; text?: string; button?: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("click", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
