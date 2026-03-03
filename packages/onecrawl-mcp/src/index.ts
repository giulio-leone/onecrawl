/**
 * @giulio-leone/onecrawl-mcp
 *
 * MCP tools server for OneCrawl browser automation — 54 tools for AI agents.
 * 13 existing browser tools + 41 generated CLI tools (M4–M9).
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
  generatedCliTools,
} from "./tools/index.js";

const existingTools = [
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
 * Create an MCP server with all 54 OneCrawl tools registered
 * (13 existing browser tools + 41 generated CLI tools).
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

  // Register the 13 existing browser tools
  for (const tool of existingTools) {
    server.tool(
      tool.name,
      tool.description,
      tool.inputSchema.shape,
      async (args: Record<string, unknown>) => tool.handler(args as never, client),
    );
  }

  // Register the 41 generated CLI tools
  for (const tool of generatedCliTools) {
    server.tool(
      tool.name,
      tool.description,
      tool.inputSchema.shape,
      async (args: Record<string, unknown>) => tool.handler(args, client),
    );
  }

  return server;
}

export { OneCrawlClient };
export type { McpServer };
