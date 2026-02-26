import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserUploadTool = {
  name: "browser_upload" as const,
  description: "Upload a file to a file input element on the page.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the file input element"),
    filePath: z.string().describe("Absolute path to the file to upload"),
  }),
  handler: async (
    args: { selector: string; filePath: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("upload", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
