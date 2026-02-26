/**
 * Storage Port - Platform-agnostic key-value storage
 *
 * Abstracts filesystem operations for cross-platform compatibility.
 * Node.js: uses fs/promises (FsStorageAdapter)
 * React Native: uses AsyncStorage/MMKV
 * Fallback: in-memory (MemoryStorageAdapter)
 */

export interface StoragePort {
  /** Retrieve a value by key. Returns null if not found. */
  get(key: string): Promise<string | null>;

  /** Store a value by key. */
  set(key: string, value: string): Promise<void>;

  /** Remove a value by key. */
  remove(key: string): Promise<void>;

  /** Check if a key exists. */
  has(key: string): Promise<boolean>;

  /** List all keys, optionally filtered by prefix. */
  keys(prefix?: string): Promise<string[]>;
}
