import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export const browserAuthStatusTool = {
  name: "browser_auth_status" as const,
  description: "Check authentication status: whether cookies are valid, passkey is available, and session is active.",
  inputSchema: z.object({}),
  handler: async (
    _args: Record<string, never>,
    client: OneCrawlClient,
  ) => {
    const result = await client.webAction("auth-status", {});
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  },
};
