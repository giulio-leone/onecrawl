import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserScreenshotTool = {
  name: "browser_screenshot" as const,
  description: "Take a screenshot of the current page or a specific element.",
  inputSchema: z.object({
    selector: z
      .string()
      .optional()
      .describe("CSS selector of the element to screenshot (omit for full viewport)"),
    fullPage: z
      .boolean()
      .optional()
      .describe("Capture the full scrollable page (default: false)"),
  }),
  handler: async (
    args: { selector?: string; fullPage?: boolean },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("screenshot", args);
    const data = result as { screenshotBase64?: string; [key: string]: unknown };

    if (data.screenshotBase64) {
      return {
        content: [
          {
            type: "image" as const,
            data: data.screenshotBase64,
            mimeType: "image/png" as const,
          },
        ],
      };
    }

    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
