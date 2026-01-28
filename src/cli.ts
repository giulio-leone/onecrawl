#!/usr/bin/env node

/**
 * OneCrawl CLI
 * Command-line interface for web crawling and scraping.
 */

import { createOneCrawl } from "./index.js";

const VERSION = "1.0.0";

interface CliArgs {
  command: "scrape" | "search" | "version" | "help";
  url?: string;
  query?: string;
  engine?: "google" | "bing" | "duckduckgo";
  maxResults?: number;
  output?: "json" | "text" | "markdown";
  browser?: boolean;
}

function parseArgs(args: string[]): CliArgs {
  const result: CliArgs = { command: "help", output: "text" };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    const next = args[i + 1];

    switch (arg) {
      case "scrape":
        result.command = "scrape";
        if (next && !next.startsWith("-")) {
          result.url = next;
          i++;
        }
        break;
      case "search":
        result.command = "search";
        if (next && !next.startsWith("-")) {
          result.query = next;
          i++;
        }
        break;
      case "version":
      case "-v":
      case "--version":
        result.command = "version";
        break;
      case "help":
      case "-h":
      case "--help":
        result.command = "help";
        break;
      case "-e":
      case "--engine":
        if (next === "google" || next === "bing" || next === "duckduckgo") {
          result.engine = next;
          i++;
        }
        break;
      case "-n":
      case "--max-results":
        if (next) {
          result.maxResults = parseInt(next, 10);
          i++;
        }
        break;
      case "-o":
      case "--output":
        if (next === "json" || next === "text" || next === "markdown") {
          result.output = next;
          i++;
        }
        break;
      case "-b":
      case "--browser":
        result.browser = true;
        break;
    }
  }

  return result;
}

function printHelp(): void {
  console.log(`
OneCrawl v${VERSION} - Native TypeScript Web Crawler

USAGE:
  onecrawl <command> [options]

COMMANDS:
  scrape <url>       Scrape content from a URL
  search <query>     Search the web
  version            Show version
  help               Show this help

OPTIONS:
  -e, --engine       Search engine: google, bing, duckduckgo (default: duckduckgo)
  -n, --max-results  Maximum search results (default: 10)
  -o, --output       Output format: json, text, markdown (default: text)
  -b, --browser      Use browser for JavaScript rendering

EXAMPLES:
  onecrawl scrape https://example.com
  onecrawl scrape https://spa.example.com --browser
  onecrawl search "TypeScript tutorial" --engine duckduckgo
  onecrawl search "AI news" -n 5 -o json
`);
}

async function runScrape(
  url: string,
  useBrowser: boolean,
  output: string,
): Promise<void> {
  const crawler = createOneCrawl();

  console.error(`Scraping: ${url}...`);

  const response = await crawler.scrape(url, {
    preferBrowser: useBrowser,
    onProgress: (event) => {
      console.error(`  [${event.phase}] ${event.message}`);
    },
  });

  const result = response.result;

  switch (output) {
    case "json":
      console.log(JSON.stringify(result, null, 2));
      break;
    case "markdown":
      console.log(`# ${result.title}\n\n${result.markdown || result.content}`);
      break;
    default:
      console.log(`Title: ${result.title}`);
      console.log(`URL: ${result.url}`);
      console.log(`---`);
      console.log(result.content);
  }
}

async function runSearch(
  query: string,
  engine: "google" | "bing" | "duckduckgo",
  maxResults: number,
  output: string,
): Promise<void> {
  const crawler = createOneCrawl();

  console.error(`Searching: "${query}" on ${engine}...`);

  const results = await crawler.search(query, {
    engine,
    maxResults,
    onProgress: (event) => {
      console.error(`  [${event.phase}] ${event.message}`);
    },
  });

  switch (output) {
    case "json":
      console.log(JSON.stringify(results, null, 2));
      break;
    default:
      console.log(`\nFound ${results.results.length} results:\n`);
      results.results.forEach((r, i) => {
        console.log(`${i + 1}. ${r.title}`);
        console.log(`   ${r.url}`);
        if (r.snippet) console.log(`   ${r.snippet}`);
        console.log();
      });
  }
}

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));

  try {
    switch (args.command) {
      case "version":
        console.log(`OneCrawl v${VERSION}`);
        break;

      case "scrape":
        if (!args.url) {
          console.error("Error: URL required for scrape command");
          process.exit(1);
        }
        await runScrape(args.url, args.browser ?? false, args.output ?? "text");
        break;

      case "search":
        if (!args.query) {
          console.error("Error: Query required for search command");
          process.exit(1);
        }
        await runSearch(
          args.query,
          args.engine ?? "duckduckgo",
          args.maxResults ?? 10,
          args.output ?? "text",
        );
        break;

      default:
        printHelp();
    }
  } catch (error) {
    console.error(
      "Error:",
      error instanceof Error ? error.message : String(error),
    );
    process.exit(1);
  }
}

main();
