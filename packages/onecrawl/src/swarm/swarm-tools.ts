/**
 * Swarm AI tool definitions for the crawl orchestrator agent.
 */

import { tool } from "ai";
import { z } from "zod";
import type { ScrapeResponse } from "../ports/index.js";
import type { SearchResults } from "../domain/schemas.js";
import type { CrawlResult, SwarmState } from "./crawl-swarm.js";

/** Minimal interface for the scrape use case. */
interface ScrapeExecutor {
  execute(url: string): Promise<ScrapeResponse>;
}

/** Minimal interface for the search use case. */
interface SearchExecutor {
  execute(
    query: string,
    options?: { maxResults?: number },
  ): Promise<SearchResults>;
}

/** Create AI tools bound to the swarm's state and use cases. */
export function createSwarmTools(
  state: SwarmState,
  scrapeUseCase: ScrapeExecutor,
  searchUseCase: SearchExecutor,
  shouldCrawl: (url: string) => boolean,
) {
  const searchWeb = tool({
    description: "Search the web for URLs related to a query",
    inputSchema: z.object({
      query: z.string().describe("Search query"),
      maxResults: z.number().default(10),
    }),
    execute: async ({ query, maxResults }) => {
      const result = await searchUseCase.execute(query, { maxResults });
      return (
        result.results?.map((r) => ({
          url: r.url,
          title: r.title,
          snippet: r.snippet,
        })) ?? []
      );
    },
  });

  const scrapeUrl = tool({
    description: "Scrape content from a URL",
    inputSchema: z.object({
      url: z.string().describe("URL to scrape"),
    }),
    execute: async ({ url }) => {
      if (state.visited.has(url)) {
        return { cached: true, url };
      }

      const response = await scrapeUseCase.execute(url);
      state.visited.add(url);

      const links =
        response.result.links?.slice(0, 20).map((l) => l.href) ?? [];
      const crawlResult: CrawlResult = {
        url,
        title: response.result.title,
        content: response.result.content.slice(0, 5000),
        links,
        relevanceScore: 0.5,
      };

      state.results.set(url, crawlResult);

      return {
        title: crawlResult.title,
        contentPreview: crawlResult.content.slice(0, 500),
        linkCount: crawlResult.links.length,
      };
    },
  });

  const addToQueue = tool({
    description: "Add URLs to crawl queue with priority",
    inputSchema: z.object({
      urls: z.array(
        z.object({
          url: z.string(),
          priority: z.number().min(0).max(10),
          reason: z.string().optional(),
        }),
      ),
    }),
    execute: async ({ urls }) => {
      let added = 0;
      for (const item of urls) {
        if (!shouldCrawl(item.url)) continue;
        if (state.visited.has(item.url)) continue;

        state.queue.push({
          url: item.url,
          priority: item.priority,
          depth: 0,
          reason: item.reason,
        });
        added++;
      }

      state.queue.sort((a, b) => b.priority - a.priority);
      return { added, queueSize: state.queue.length };
    },
  });

  const getQueueStatus = tool({
    description: "Get current queue and crawl status",
    inputSchema: z.object({}),
    execute: async () => ({
      queueSize: state.queue.length,
      visited: state.visited.size,
      resultsCount: state.results.size,
      nextUrls: state.queue.slice(0, 5).map((u) => u.url),
    }),
  });

  const getResults = tool({
    description: "Get all crawled results",
    inputSchema: z.object({}),
    execute: async () => {
      return [...state.results.values()].map((r) => ({
        url: r.url,
        title: r.title,
        contentPreview: r.content.slice(0, 200),
      }));
    },
  });

  const finishCrawl = tool({
    description: "Call when you have gathered enough content for the goal",
    inputSchema: z.object({
      summary: z.string().describe("Summary of what was found"),
    }),
    execute: async ({ summary }) => ({ finished: true, summary }),
  });

  return {
    searchWeb,
    scrapeUrl,
    addToQueue,
    getQueueStatus,
    getResults,
    finishCrawl,
  };
}
