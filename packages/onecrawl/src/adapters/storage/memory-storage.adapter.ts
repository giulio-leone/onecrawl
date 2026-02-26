/**
 * Memory Storage Adapter
 *
 * In-memory implementation of StoragePort.
 * Works on all platforms as a universal fallback.
 * Data does not persist across process restarts.
 */

import type { StoragePort } from "../../ports/storage.port.js";

export class MemoryStorageAdapter implements StoragePort {
  private data = new Map<string, string>();

  async get(key: string): Promise<string | null> {
    return this.data.get(key) ?? null;
  }

  async set(key: string, value: string): Promise<void> {
    this.data.set(key, value);
  }

  async remove(key: string): Promise<void> {
    this.data.delete(key);
  }

  async has(key: string): Promise<boolean> {
    return this.data.has(key);
  }

  async keys(prefix?: string): Promise<string[]> {
    const allKeys = [...this.data.keys()];
    if (!prefix) return allKeys;
    return allKeys.filter((k) => k.startsWith(prefix));
  }
}

/** Factory function for MemoryStorageAdapter */
export function createMemoryStorageAdapter(): MemoryStorageAdapter {
  return new MemoryStorageAdapter();
}
