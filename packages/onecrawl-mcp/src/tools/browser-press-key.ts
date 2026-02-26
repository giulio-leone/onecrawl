import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserPressKeyTool = {
  name: "browser_press_key" as const,
  description: "Press a keyboard key (e.g. Enter, Escape, Tab, ArrowDown).",
  inputSchema: z.object({
    key: z.string().describe("Key to press (e.g. 'Enter', 'Escape', 'Tab', 'ArrowDown')"),
  }),
  handler: async (
    args: { key: string },
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("press", args);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
