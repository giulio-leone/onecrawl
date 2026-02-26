/**
 * AsyncStorage Adapter (React Native)
 *
 * StoragePort implementation using @react-native-async-storage/async-storage.
 * Consumer injects the AsyncStorage instance via constructor.
 *
 * Usage:
 *   import AsyncStorage from '@react-native-async-storage/async-storage';
 *   const storage = new AsyncStorageAdapter(AsyncStorage);
 */

import type { StoragePort } from "../../ports/storage.port.js";

const DEFAULT_PREFIX = "@onecrawl:";

/** Minimal interface matching AsyncStorage's API surface */
export interface AsyncStorageStatic {
  getItem(key: string): Promise<string | null>;
  setItem(key: string, value: string): Promise<void>;
  removeItem(key: string): Promise<void>;
  getAllKeys(): Promise<readonly string[]>;
}

export class AsyncStorageAdapter implements StoragePort {
  private prefix: string;

  constructor(
    private storage: AsyncStorageStatic,
    options?: { prefix?: string },
  ) {
    this.prefix = options?.prefix ?? DEFAULT_PREFIX;
  }

  private prefixedKey(key: string): string {
    return `${this.prefix}${key}`;
  }

  private unprefixedKey(key: string): string {
    return key.slice(this.prefix.length);
  }

  async get(key: string): Promise<string | null> {
    return this.storage.getItem(this.prefixedKey(key));
  }

  async set(key: string, value: string): Promise<void> {
    await this.storage.setItem(this.prefixedKey(key), value);
  }

  async remove(key: string): Promise<void> {
    await this.storage.removeItem(this.prefixedKey(key));
  }

  async has(key: string): Promise<boolean> {
    const value = await this.storage.getItem(this.prefixedKey(key));
    return value !== null;
  }

  async keys(prefix?: string): Promise<string[]> {
    const allKeys = await this.storage.getAllKeys();
    const ownKeys = allKeys.filter((k) => k.startsWith(this.prefix));
    const unprefixed = ownKeys.map((k) => this.unprefixedKey(k));
    if (!prefix) return unprefixed;
    return unprefixed.filter((k) => k.startsWith(prefix));
  }
}

/** Factory function for AsyncStorageAdapter */
export function createAsyncStorageAdapter(
  storage: AsyncStorageStatic,
  options?: { prefix?: string },
): AsyncStorageAdapter {
  return new AsyncStorageAdapter(storage, options);
}
