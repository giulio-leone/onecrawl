/**
 * Semantic Crawl Use Case
 * Discovers interactive UI tools by crawling a site's pages.
 */

import type { ScraperPort } from "../ports/index.js";
import type {
  CrawlTarget,
  CrawlProgress,
  SemanticCrawlResult,
  SemanticTool,
} from "../domain/semantic-tool.js";
import {
  extractToolsFromHTML,
  extractInternalLinks,
  matchesPatterns,
} from "../utils/semantic-extractor.js";

export class SemanticCrawlUseCase {
  private scraper: ScraperPort;
  private running = false;
  private abortController: AbortController | null = null;

  constructor(scraper: ScraperPort) {
    this.scraper = scraper;
  }

  async execute(
    target: CrawlTarget,
    onProgress?: (p: CrawlProgress) => void,
  ): Promise<SemanticCrawlResult> {
    this.running = true;
    this.abortController = new AbortController();
    const { signal } = this.abortController;

    const startTime = Date.now();
    const visited = new Set<string>();
    const toolsByPage = new Map<string, SemanticTool[]>();
    const errors: string[] = [];

    // Queue entries: [url, depth]
    const queue: Array<[string, number]> = target.entryPoints.map((u) => [
      u,
      0,
    ]);
    let totalToolsFound = 0;

    while (queue.length > 0 && visited.size < target.maxPages) {
      if (signal.aborted) break;

      const [url, depth] = queue.shift()!;
      if (visited.has(url)) continue;
      visited.add(url);

      // Filter by include/exclude patterns
      if (
        target.includePatterns &&
        target.includePatterns.length > 0 &&
        !matchesPatterns(url, target.includePatterns)
      ) {
        continue;
      }
      if (
        target.excludePatterns &&
        target.excludePatterns.length > 0 &&
        matchesPatterns(url, target.excludePatterns)
      ) {
        continue;
      }

      try {
        const response = await this.scraper.scrape(url, { signal });
        const html = response.result.html ?? response.result.content;
        if (!html) continue;

        const tools = extractToolsFromHTML(html, url);
        if (tools.length > 0) {
          toolsByPage.set(url, tools);
          totalToolsFound += tools.length;
        }

        onProgress?.({
          pagesScanned: visited.size,
          pagesTotal: Math.min(
            visited.size + queue.length,
            target.maxPages,
          ),
          currentUrl: url,
          toolsFound: totalToolsFound,
          errors: errors.length,
        });

        // Follow internal links if within depth limit
        if (depth < target.maxDepth) {
          const links = extractInternalLinks(html, url);
          for (const link of links) {
            if (!visited.has(link) && visited.size + queue.length < target.maxPages) {
              queue.push([link, depth + 1]);
            }
          }
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        errors.push(`${url}: ${msg}`);
      }
    }

    this.running = false;
    this.abortController = null;

    return {
      site: target.site,
      pagesScanned: visited.size,
      toolsDiscovered: totalToolsFound,
      toolsByPage,
      duration: Date.now() - startTime,
      errors,
    };
  }

  cancel(): void {
    this.abortController?.abort();
    this.running = false;
  }

  isRunning(): boolean {
    return this.running;
  }
}
