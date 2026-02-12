/**
 * Crawl Swarm - Distributed crawling with AI SDK v6 ToolLoopAgent
 *
 * Architecture:
 * - Orchestrator Agent: Plans crawl strategy, prioritizes URLs
 * - Uses ToolLoopAgent for automatic tool loop
 * - Coordinator: Manages queue, deduplication, rate limiting
 */

import { ToolLoopAgent, tool, stepCountIs, type LanguageModel } from "ai";
import { z } from "zod";
import { createScrapeUseCase } from "../use-cases/scrape.use-case.js";
import { createSearchUseCase } from "../use-cases/search.use-case.js";

/** URL with priority and metadata */
export interface PrioritizedUrl {
  url: string;
  priority: number;
  depth: number;
  parentUrl?: string;
  reason?: string;
}

/** Crawl result */
export interface CrawlResult {
  url: string;
  title: string;
  content: string;
  links: string[];
  relevanceScore: number;
}

/** Swarm configuration */
export interface SwarmConfig {
  /** AI model for agents */
  model: LanguageModel;
  /** Maximum concurrent workers */
  maxWorkers?: number;
  /** Maximum URLs to crawl */
  maxUrls?: number;
  /** Maximum depth */
  maxDepth?: number;
  /** Domains to stay within (empty = all) */
  allowedDomains?: string[];
  /** URL patterns to exclude */
  excludePatterns?: RegExp[];
  /** Maximum steps per agent */
  maxSteps?: number;
}

/** Swarm state */
interface SwarmState {
  queue: PrioritizedUrl[];
  visited: Set<string>;
  results: Map<string, CrawlResult>;
  activeWorkers: number;
}

/**
 * CrawlSwarm - Agentic distributed crawling with ToolLoopAgent
 */
export class CrawlSwarm {
  private config: Required<SwarmConfig>;
  private state: SwarmState;
  private scrapeUseCase = createScrapeUseCase();
  private searchUseCase = createSearchUseCase();

  constructor(config: SwarmConfig) {
    this.config = {
      maxWorkers: 5,
      maxUrls: 100,
      maxDepth: 3,
      allowedDomains: [],
      excludePatterns: [],
      maxSteps: 20,
      ...config,
    };

    this.state = {
      queue: [],
      visited: new Set(),
      results: new Map(),
      activeWorkers: 0,
    };
  }

  private shouldCrawl(url: string): boolean {
    try {
      const parsed = new URL(url);

      if (this.config.allowedDomains.length > 0) {
        const domain = parsed.hostname.replace("www.", "");
        if (!this.config.allowedDomains.some((d) => domain.includes(d))) {
          return false;
        }
      }

      for (const pattern of this.config.excludePatterns) {
        if (pattern.test(url)) return false;
      }

      if (this.state.visited.size >= this.config.maxUrls) return false;

      return true;
    } catch {
      return false;
    }
  }

  /**
   * Execute a goal-directed crawl using ToolLoopAgent
   */
  async crawl(
    goal: string,
    options: {
      seedUrls?: string[];
      onProgress?: (status: {
        visited: number;
        queued: number;
        step: string;
      }) => void;
    } = {},
  ): Promise<{
    results: CrawlResult[];
    summary: string;
  }> {
    const { seedUrls = [], onProgress } = options;

    // Add seed URLs to queue
    for (const url of seedUrls) {
      if (this.shouldCrawl(url)) {
        this.state.queue.push({ url, priority: 10, depth: 0 });
      }
    }

    // Create tools bound to this instance
    const searchWeb = tool({
      description: "Search the web for URLs related to a query",
      inputSchema: z.object({
        query: z.string().describe("Search query"),
        maxResults: z.number().default(10),
      }),
      execute: async ({ query, maxResults }) => {
        const result = await this.searchUseCase.execute(query, { maxResults });
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
        if (this.state.visited.has(url)) {
          return { cached: true, url };
        }

        const response = await this.scrapeUseCase.execute(url);
        this.state.visited.add(url);

        const links =
          response.result.links?.slice(0, 20).map((l) => l.href) ?? [];
        const crawlResult: CrawlResult = {
          url,
          title: response.result.title,
          content: response.result.content.slice(0, 5000),
          links,
          relevanceScore: 0.5,
        };

        this.state.results.set(url, crawlResult);

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
          if (!this.shouldCrawl(item.url)) continue;
          if (this.state.visited.has(item.url)) continue;

          this.state.queue.push({
            url: item.url,
            priority: item.priority,
            depth: 0,
            reason: item.reason,
          });
          added++;
        }

        this.state.queue.sort((a, b) => b.priority - a.priority);
        return { added, queueSize: this.state.queue.length };
      },
    });

    const getQueueStatus = tool({
      description: "Get current queue and crawl status",
      inputSchema: z.object({}),
      execute: async () => ({
        queueSize: this.state.queue.length,
        visited: this.state.visited.size,
        resultsCount: this.state.results.size,
        nextUrls: this.state.queue.slice(0, 5).map((u) => u.url),
      }),
    });

    const getResults = tool({
      description: "Get all crawled results",
      inputSchema: z.object({}),
      execute: async () => {
        return [...this.state.results.values()].map((r) => ({
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

    // Create orchestrator agent with ToolLoopAgent
    const orchestrator = new ToolLoopAgent({
      model: this.config.model,
      instructions: `You are a crawl orchestrator. Your job is to:
1. Analyze the crawl goal and plan a strategy
2. Prioritize URLs based on relevance to the goal
3. Scrape relevant pages and extract information
4. Call finishCrawl when you have gathered enough content

Always explain your reasoning briefly before taking action.`,
      tools: {
        searchWeb,
        scrapeUrl,
        addToQueue,
        getQueueStatus,
        getResults,
        finishCrawl,
      },
      stopWhen: stepCountIs(this.config.maxSteps),
      onStepFinish: (stepResult) => {
        onProgress?.({
          visited: this.state.visited.size,
          queued: this.state.queue.length,
          step:
            stepResult.toolCalls?.map((tc) => tc.toolName).join(", ") ??
            "thinking",
        });
      },
    });

    // Run the agent
    const prompt =
      seedUrls.length > 0
        ? `Goal: ${goal}\n\nSeed URLs: ${seedUrls.join(", ")}\n\nCrawl these URLs and find relevant content.`
        : `Goal: ${goal}\n\nSearch the web and crawl relevant pages to gather information.`;

    const result = await orchestrator.generate({ prompt });

    return {
      results: [...this.state.results.values()],
      summary: result.text,
    };
  }

  /** Reset swarm state */
  reset(): void {
    this.state = {
      queue: [],
      visited: new Set(),
      results: new Map(),
      activeWorkers: 0,
    };
  }
}

/**
 * Create a crawl swarm
 */
export function createCrawlSwarm(config: SwarmConfig): CrawlSwarm {
  return new CrawlSwarm(config);
}
