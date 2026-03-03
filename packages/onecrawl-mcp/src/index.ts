/**
 * @giulio-leone/onecrawl-mcp
 *
 * MCP tools server for OneCrawl browser automation — 13 tools for AI agents.
 * Connects to an onecrawl-server instance via HTTP using @giulio-leone/onecrawl-client.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { OneCrawlClient } from "@giulio-leone/onecrawl-client";

import {
  browserNavigateTool,
  browserClickTool,
  browserTypeTool,
  browserScreenshotTool,
  browserExtractTool,
  webScrapeTool,
  browserEvaluateTool,
  browserWaitTool,
  browserPressKeyTool,
  browserUploadTool,
  browserHumanClickTool,
  browserAuthStatusTool,
  browserHealthCheckTool,
} from "./tools/index.js";

const allTools = [
  browserNavigateTool,
  browserClickTool,
  browserTypeTool,
  browserScreenshotTool,
  browserExtractTool,
  webScrapeTool,
  browserEvaluateTool,
  browserWaitTool,
  browserPressKeyTool,
  browserUploadTool,
  browserHumanClickTool,
  browserAuthStatusTool,
  browserHealthCheckTool,
] as const;

export interface CreateServerOptions {
  /** OneCrawl server URL (default: ONECRAWL_URL env or http://localhost:4100) */
  onecrawlUrl?: string;
}

/**
 * Create an MCP server with all 13 OneCrawl browser automation tools registered.
 */
export function createServer(options: CreateServerOptions = {}): McpServer {
  const url =
    options.onecrawlUrl ??
    process.env.ONECRAWL_URL ??
    "http://localhost:4100";

  const client = new OneCrawlClient(url);

  const server = new McpServer({
    name: "onecrawl-mcp",
    version: "1.0.0",
  });

  for (const tool of allTools) {
    server.tool(
      tool.name,
      tool.description,
      tool.inputSchema.shape,
      async (args: Record<string, unknown>) => tool.handler(args as never, client),
    );
  }

  return server;
}

export { OneCrawlClient };
export type { McpServer };
