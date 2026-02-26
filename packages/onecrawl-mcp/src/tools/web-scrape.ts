import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const webScrapeTool = {
  name: "web_scrape" as const,
  description: "Scrape a full page and return its content in the specified format.",
  inputSchema: z.object({
    url: z.string().url().describe("URL of the page to scrape"),
    format: z
      .enum(["markdown", "text", "html"])
      .optional()
      .describe("Output format (default: markdown)"),
  }),
  handler: async (
    args: { url: string; format?: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.ingest(args.url, {});
    const text =
      args.format === "html"
        ? (result as Record<string, unknown>).html ?? result.content
        : result.content;
    return {
      content: [{ type: "text" as const, text: String(text) }],
    };
  },
};
