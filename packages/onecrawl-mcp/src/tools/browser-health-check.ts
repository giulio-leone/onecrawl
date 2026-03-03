import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserHealthCheckTool = {
  name: "browser_health_check" as const,
  description: "Check browser health: whether the browser is alive, page is responsive, and stealth patches are active.",
  inputSchema: z.object({}),
  handler: async (
    _args: Record<string, never>,
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("health-check", {});
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
