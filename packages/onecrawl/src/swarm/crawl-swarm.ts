/**
 * Crawl Swarm - Distributed crawling with AI SDK v6 ToolLoopAgent
 *
 * Architecture:
 * - Orchestrator Agent: Plans crawl strategy, prioritizes URLs
 * - Uses ToolLoopAgent for automatic tool loop
 * - Coordinator: Manages queue, deduplication, rate limiting
 */

import { ToolLoopAgent, stepCountIs, type LanguageModel } from "ai";
import { createScrapeUseCase } from "../use-cases/scrape.use-case.js";
import { createSearchUseCase } from "../use-cases/search.use-case.js";
import { createSwarmTools } from "./swarm-tools.js";

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
  model: LanguageModel;
  maxWorkers?: number;
  maxUrls?: number;
  maxDepth?: number;
  allowedDomains?: string[];
  excludePatterns?: RegExp[];
  maxSteps?: number;
}

/** Swarm state */
export interface SwarmState {
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

    this.state = this.createInitialState();
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

  /** Execute a goal-directed crawl using ToolLoopAgent. */
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
  ): Promise<{ results: CrawlResult[]; summary: string }> {
    const { seedUrls = [], onProgress } = options;

    for (const url of seedUrls) {
      if (this.shouldCrawl(url)) {
        this.state.queue.push({ url, priority: 10, depth: 0 });
      }
    }

    const tools = createSwarmTools(
      this.state,
      this.scrapeUseCase,
      this.searchUseCase,
      this.shouldCrawl.bind(this),
    );

    const orchestrator = new ToolLoopAgent({
      model: this.config.model,
      instructions: `You are a crawl orchestrator. Your job is to:
1. Analyze the crawl goal and plan a strategy
2. Prioritize URLs based on relevance to the goal
3. Scrape relevant pages and extract information
4. Call finishCrawl when you have gathered enough content

Always explain your reasoning briefly before taking action.`,
      tools,
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

  /** Reset swarm state. */
  reset(): void {
    this.state = this.createInitialState();
  }

  private createInitialState(): SwarmState {
    return {
      queue: [],
      visited: new Set(),
      results: new Map(),
      activeWorkers: 0,
    };
  }
}

/** Create a crawl swarm. */
export function createCrawlSwarm(config: SwarmConfig): CrawlSwarm {
  return new CrawlSwarm(config);
}
