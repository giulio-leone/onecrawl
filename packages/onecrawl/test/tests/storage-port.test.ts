/**
 * Storage Port Tests
 * Tests the MemoryStorageAdapter against the StoragePort contract.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { MemoryStorageAdapter } from "../../src/adapters/storage/memory-storage.adapter.js";

describe("MemoryStorageAdapter", () => {
  let storage: MemoryStorageAdapter;

  beforeEach(() => {
    storage = new MemoryStorageAdapter();
  });

  it("should set and get a value", async () => {
    await storage.set("key1", "value1");
    expect(await storage.get("key1")).toBe("value1");
  });

  it("should return null for missing key", async () => {
    expect(await storage.get("missing")).toBeNull();
  });

  it("should check if key exists", async () => {
    await storage.set("key1", "value1");
    expect(await storage.has("key1")).toBe(true);
    expect(await storage.has("missing")).toBe(false);
  });

  it("should remove a key", async () => {
    await storage.set("key1", "value1");
    await storage.remove("key1");
    expect(await storage.has("key1")).toBe(false);
  });

  it("should return null after removing a key", async () => {
    await storage.set("key1", "value1");
    await storage.remove("key1");
    expect(await storage.get("key1")).toBeNull();
  });

  it("should list keys with prefix", async () => {
    await storage.set("cookies:google.com", "...");
    await storage.set("cookies:github.com", "...");
    await storage.set("settings:theme", "dark");

    const cookieKeys = await storage.keys("cookies:");
    expect(cookieKeys).toHaveLength(2);
    expect(cookieKeys).toContain("cookies:google.com");
    expect(cookieKeys).toContain("cookies:github.com");
  });

  it("should list all keys when no prefix", async () => {
    await storage.set("a", "1");
    await storage.set("b", "2");
    const keys = await storage.keys();
    expect(keys).toHaveLength(2);
  });

  it("should overwrite existing values", async () => {
    await storage.set("key1", "old");
    await storage.set("key1", "new");
    expect(await storage.get("key1")).toBe("new");
  });

  it("should handle empty prefix returning all keys", async () => {
    await storage.set("x", "1");
    await storage.set("y", "2");
    const keys = await storage.keys("");
    expect(keys).toHaveLength(2);
  });

  it("should not fail when removing non-existent key", async () => {
    await expect(storage.remove("nonexistent")).resolves.toBeUndefined();
  });
});
