/**
 * Filesystem Storage Adapter
 *
 * Node.js implementation of StoragePort using fs/promises.
 * Stores each key as a file in a configurable base directory.
 * Keys are sanitized to safe filenames.
 */

import {
  readFile,
  writeFile,
  unlink,
  readdir,
  mkdir,
  access,
} from "fs/promises";
import { join } from "path";
import { homedir } from "os";

import type { StoragePort } from "../../ports/storage.port.js";

/** Sanitize a key to a safe filename */
function keyToFilename(key: string): string {
  return key.replace(/[^a-zA-Z0-9._-]/g, "_");
}

export class FsStorageAdapter implements StoragePort {
  private baseDir: string;
  private initialized = false;

  constructor(baseDir?: string) {
    this.baseDir = baseDir ?? join(homedir(), ".onecrawl", "storage");
  }

  /** Ensure the storage directory exists */
  private async ensureDir(): Promise<void> {
    if (this.initialized) return;
    await mkdir(this.baseDir, { recursive: true });
    this.initialized = true;
  }

  async get(key: string): Promise<string | null> {
    try {
      const filePath = join(this.baseDir, keyToFilename(key));
      return await readFile(filePath, "utf-8");
    } catch {
      return null;
    }
  }

  async set(key: string, value: string): Promise<void> {
    await this.ensureDir();
    const filePath = join(this.baseDir, keyToFilename(key));
    await writeFile(filePath, value, "utf-8");
  }

  async remove(key: string): Promise<void> {
    try {
      const filePath = join(this.baseDir, keyToFilename(key));
      await unlink(filePath);
    } catch {
      // Ignore if file doesn't exist
    }
  }

  async has(key: string): Promise<boolean> {
    try {
      const filePath = join(this.baseDir, keyToFilename(key));
      await access(filePath);
      return true;
    } catch {
      return false;
    }
  }

  async keys(prefix?: string): Promise<string[]> {
    await this.ensureDir();
    const files = await readdir(this.baseDir);
    if (!prefix) return files;
    const safePrefix = keyToFilename(prefix);
    return files.filter((f) => f.startsWith(safePrefix));
  }
}

/** Factory function for FsStorageAdapter */
export function createFsStorageAdapter(baseDir?: string): FsStorageAdapter {
  return new FsStorageAdapter(baseDir);
}
