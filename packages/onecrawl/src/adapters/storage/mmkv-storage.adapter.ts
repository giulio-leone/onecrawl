/**
 * MMKV Storage Adapter (React Native)
 *
 * StoragePort implementation using react-native-mmkv.
 * MMKV is synchronous — methods are wrapped in Promise.resolve().
 * Consumer injects the MMKV instance via constructor.
 *
 * Usage:
 *   import { MMKV } from 'react-native-mmkv';
 *   const mmkv = new MMKV();
 *   const storage = new MmkvStorageAdapter(mmkv);
 */

import type { StoragePort } from "../../ports/storage.port.js";

const DEFAULT_PREFIX = "@onecrawl:";

/** Minimal interface matching MMKV's API surface */
export interface MmkvInstance {
  getString(key: string): string | undefined;
  set(key: string, value: string): void;
  delete(key: string): void;
  contains(key: string): boolean;
  getAllKeys(): string[];
}

export class MmkvStorageAdapter implements StoragePort {
  private prefix: string;

  constructor(
    private storage: MmkvInstance,
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
    return Promise.resolve(
      this.storage.getString(this.prefixedKey(key)) ?? null,
    );
  }

  async set(key: string, value: string): Promise<void> {
    this.storage.set(this.prefixedKey(key), value);
    return Promise.resolve();
  }

  async remove(key: string): Promise<void> {
    this.storage.delete(this.prefixedKey(key));
    return Promise.resolve();
  }

  async has(key: string): Promise<boolean> {
    return Promise.resolve(this.storage.contains(this.prefixedKey(key)));
  }

  async keys(prefix?: string): Promise<string[]> {
    const allKeys = this.storage.getAllKeys();
    const ownKeys = allKeys.filter((k) => k.startsWith(this.prefix));
    const unprefixed = ownKeys.map((k) => this.unprefixedKey(k));
    if (!prefix) return Promise.resolve(unprefixed);
    return Promise.resolve(unprefixed.filter((k) => k.startsWith(prefix)));
  }
}

/** Factory function for MmkvStorageAdapter */
export function createMmkvStorageAdapter(
  storage: MmkvInstance,
  options?: { prefix?: string },
): MmkvStorageAdapter {
  return new MmkvStorageAdapter(storage, options);
}
