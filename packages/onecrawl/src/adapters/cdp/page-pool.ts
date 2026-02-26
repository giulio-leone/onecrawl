/**
 * CDP page pool management.
 * Manages a pool of warm CDP pages for reuse across scrape calls.
 */

import { getRandomUserAgent, getRandomViewport } from "../../utils/stealth.js";
import { CDPClient, CDPPage } from "./client.js";

/** A pooled page with usage tracking. */
interface PooledPage {
  page: CDPPage;
  inUse: boolean;
  lastUsed: number;
}

/** Manages a pool of warm CDP pages for concurrent scraping. */
export class PagePool {
  private pool: PooledPage[] = [];
  private client: CDPClient | null = null;
  private maxSize: number;

  constructor(maxSize: number) {
    this.maxSize = maxSize;
  }

  async ensureClient(): Promise<CDPClient> {
    if (!this.client) {
      this.client = new CDPClient();
      await this.client.launch();
    }
    return this.client;
  }

  /** Acquire a page from the pool, creating one if needed. */
  async acquire(): Promise<CDPPage> {
    for (const pooled of this.pool) {
      if (!pooled.inUse) {
        pooled.inUse = true;
        pooled.lastUsed = Date.now();
        return pooled.page;
      }
    }

    if (this.pool.length < this.maxSize) {
      const client = await this.ensureClient();
      const pageInfo = await client.newPage();
      const page = new CDPPage(pageInfo, client);
      await page.connect();

      const viewport = getRandomViewport();
      await page.setViewport(viewport.width, viewport.height);
      await page.setUserAgent(getRandomUserAgent());

      this.pool.push({ page, inUse: true, lastUsed: Date.now() });
      return page;
    }

    return new Promise((resolve) => {
      const check = () => {
        for (const pooled of this.pool) {
          if (!pooled.inUse) {
            pooled.inUse = true;
            pooled.lastUsed = Date.now();
            resolve(pooled.page);
            return;
          }
        }
        setTimeout(check, 50);
      };
      check();
    });
  }

  /** Release a page back to the pool. */
  release(page: CDPPage): void {
    for (const pooled of this.pool) {
      if (pooled.page === page) {
        pooled.inUse = false;
        return;
      }
    }
  }

  /** Close all pages and the underlying client. */
  async closeAll(): Promise<void> {
    for (const pooled of this.pool) await pooled.page.close();
    this.pool = [];
    if (this.client) {
      await this.client.close();
      this.client = null;
    }
  }
}
