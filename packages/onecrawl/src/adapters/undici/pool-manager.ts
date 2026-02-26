/**
 * Undici connection pool management.
 * Creates and manages per-origin HTTP/2 connection pools.
 */

import { Pool } from "undici";

/** Manages per-origin Undici connection pools. */
export class PoolManager {
  private pools = new Map<string, Pool>();

  getPool(origin: string): Pool {
    let pool = this.pools.get(origin);
    if (!pool) {
      pool = new Pool(origin, {
        connections: 10,
        pipelining: 6,
        keepAliveTimeout: 30000,
        keepAliveMaxTimeout: 60000,
      });
      this.pools.set(origin, pool);
    }
    return pool;
  }

  async closeAll(): Promise<void> {
    const closePromises = [...this.pools.values()].map((pool) => pool.close());
    await Promise.all(closePromises);
    this.pools.clear();
  }
}
