import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserHumanClickTool = {
  name: "browser_human_click" as const,
  description:
    "Perform a human-like click with realistic mouse movement (anti-detection). Uses ghost cursor for natural behavior.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the element to click"),
  }),
  handler: async (
    args: { selector: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("human/click", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
