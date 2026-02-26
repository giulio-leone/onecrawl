/**
 * Semantic Tool Domain Types
 * Types for discovering interactive UI tools from crawled pages.
 */

import { z } from "zod";

export const SemanticToolSchema = z.object({
  name: z.string(),
  description: z.string(),
  inputSchema: z.object({
    type: z.literal("object"),
    properties: z.record(
      z.string(),
      z.object({
        type: z.enum(["string", "number", "boolean", "object", "array"]),
        description: z.string().optional(),
      }),
    ),
    required: z.array(z.string()).optional(),
  }),
  confidence: z.number().min(0).max(1).optional(),
  category: z.string().optional(),
});

export type SemanticTool = z.infer<typeof SemanticToolSchema>;

export const CrawlTargetSchema = z.object({
  site: z.string(),
  entryPoints: z.array(z.string().url("Must be a valid URL")),
  maxPages: z.number().positive().optional().default(50),
  maxDepth: z.number().nonnegative().optional().default(3),
  includePatterns: z.array(z.string()).optional(),
  excludePatterns: z.array(z.string()).optional(),
});

export type CrawlTarget = z.infer<typeof CrawlTargetSchema>;

export interface CrawlProgress {
  readonly pagesScanned: number;
  readonly pagesTotal: number;
  readonly currentUrl: string;
  readonly toolsFound: number;
  readonly errors: number;
}

export interface SemanticCrawlResult {
  readonly site: string;
  readonly pagesScanned: number;
  readonly toolsDiscovered: number;
  readonly toolsByPage: ReadonlyMap<string, readonly SemanticTool[]>;
  readonly duration: number;
  readonly errors: readonly string[];
}
