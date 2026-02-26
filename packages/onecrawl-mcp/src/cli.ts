#!/usr/bin/env node

/**
 * CLI entry point — starts the OneCrawl MCP server via stdio transport.
 *
 * Usage:
 *   ONECRAWL_URL=http://localhost:4100 npx onecrawl-mcp
 */

import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createServer } from "./index.js";

async function main() {
  const server = createServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("OneCrawl MCP server running on stdio");
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
