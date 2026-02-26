/**
 * Storage Integration Tests
 *
 * Integration-style tests for FsStorageAdapter, MemoryStorageAdapter,
 * StoragePort interchangeability, and cookie import with StoragePort.
 */

import { describe, it, expect, beforeEach, afterAll } from "vitest";
import { rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

import { FsStorageAdapter } from "../../src/adapters/storage/fs-storage.adapter.js";
import { MemoryStorageAdapter } from "../../src/adapters/storage/memory-storage.adapter.js";
import type { StoragePort } from "../../src/ports/storage.port.js";
import {
  parseCookiesTxt,
  importCookiesTxtFromStorage,
} from "../../src/auth/cookies.js";

const TEST_BASE_DIR = join(
  tmpdir(),
  `onecrawl-storage-integration-test-${Date.now()}`,
);

afterAll(async () => {
  await rm(TEST_BASE_DIR, { recursive: true, force: true });
});

// ── FsStorageAdapter Integration ──────────────────────────────────────

describe("FsStorageAdapter integration", () => {
  let storage: FsStorageAdapter;

  beforeEach(() => {
    const dir = join(TEST_BASE_DIR, `fs-${Date.now()}-${Math.random()}`);
    storage = new FsStorageAdapter(dir);
  });

  it("should store and retrieve complex JSON objects", async () => {
    const complex = {
      users: [
        { id: 1, name: "Alice", tags: ["admin", "user"] },
        { id: 2, name: "Bob", nested: { deep: { value: true } } },
      ],
      metadata: { version: "1.0", count: 42, nullable: null },
    };
    const json = JSON.stringify(complex);

    await storage.set("complex-json", json);
    const retrieved = await storage.get("complex-json");

    expect(retrieved).toBe(json);
    expect(JSON.parse(retrieved!)).toEqual(complex);
  });

  it("should store and retrieve base64 encoded binary-like strings", async () => {
    // Generate a base64 string from random-like bytes
    const binaryData = Buffer.from(
      Array.from({ length: 256 }, (_, i) => i),
    ).toString("base64");

    await storage.set("binary-data", binaryData);
    const retrieved = await storage.get("binary-data");

    expect(retrieved).toBe(binaryData);
  });

  it("should handle keys with special characters (slashes, dots)", async () => {
    const keys = [
      "path/to/resource",
      "file.name.ext",
      "domain:port/path",
      "key with spaces",
      "special!@#$%^&*()",
    ];

    for (const key of keys) {
      await storage.set(key, `value-for-${key}`);
    }

    for (const key of keys) {
      const value = await storage.get(key);
      expect(value).toBe(`value-for-${key}`);
    }
  });

  it("should handle large values (10KB+)", async () => {
    const largeValue = "x".repeat(15_000); // 15KB

    await storage.set("large-key", largeValue);
    const retrieved = await storage.get("large-key");

    expect(retrieved).toBe(largeValue);
    expect(retrieved!.length).toBe(15_000);
  });

  it("should handle concurrent read/write operations without corruption", async () => {
    const concurrency = 20;
    const writes = Array.from({ length: concurrency }, (_, i) =>
      storage.set(`concurrent-${i}`, `value-${i}`),
    );
    await Promise.all(writes);

    const reads = Array.from({ length: concurrency }, (_, i) =>
      storage.get(`concurrent-${i}`),
    );
    const results = await Promise.all(reads);

    for (let i = 0; i < concurrency; i++) {
      expect(results[i]).toBe(`value-${i}`);
    }
  });
});

// ── MemoryStorageAdapter ──────────────────────────────────────────────

describe("MemoryStorageAdapter", () => {
  it("should isolate data between separate instances", async () => {
    const storage1 = new MemoryStorageAdapter();
    const storage2 = new MemoryStorageAdapter();

    await storage1.set("shared-key", "value-from-1");
    await storage2.set("shared-key", "value-from-2");

    expect(await storage1.get("shared-key")).toBe("value-from-1");
    expect(await storage2.get("shared-key")).toBe("value-from-2");

    expect(await storage1.has("shared-key")).toBe(true);
    expect(await storage2.has("shared-key")).toBe(true);
  });

  it("should clear all data when all keys are removed", async () => {
    const storage = new MemoryStorageAdapter();

    await storage.set("a", "1");
    await storage.set("b", "2");
    await storage.set("c", "3");

    // Clear by removing all keys
    const allKeys = await storage.keys();
    for (const key of allKeys) {
      await storage.remove(key);
    }

    expect(await storage.keys()).toHaveLength(0);
    expect(await storage.get("a")).toBeNull();
    expect(await storage.get("b")).toBeNull();
    expect(await storage.get("c")).toBeNull();
  });

  it("should support iterator-like behavior with keys() and prefix filtering", async () => {
    const storage = new MemoryStorageAdapter();

    await storage.set("users:alice", "data-alice");
    await storage.set("users:bob", "data-bob");
    await storage.set("users:charlie", "data-charlie");
    await storage.set("sessions:s1", "session-data-1");
    await storage.set("sessions:s2", "session-data-2");

    const userKeys = await storage.keys("users:");
    expect(userKeys).toHaveLength(3);
    expect(userKeys.sort()).toEqual([
      "users:alice",
      "users:bob",
      "users:charlie",
    ]);

    const sessionKeys = await storage.keys("sessions:");
    expect(sessionKeys).toHaveLength(2);

    // Iterate over keys and collect values
    const userData: Record<string, string> = {};
    for (const key of userKeys) {
      userData[key] = (await storage.get(key))!;
    }
    expect(userData["users:alice"]).toBe("data-alice");
    expect(userData["users:bob"]).toBe("data-bob");
    expect(userData["users:charlie"]).toBe("data-charlie");
  });
});

// ── StoragePort Interchangeability ────────────────────────────────────

describe("StoragePort interchangeability", () => {
  function runPortContract(name: string, factory: () => StoragePort) {
    describe(name, () => {
      let storage: StoragePort;

      beforeEach(() => {
        storage = factory();
      });

      it("should set, get, has, remove consistently", async () => {
        await storage.set("key", "value");
        expect(await storage.get("key")).toBe("value");
        expect(await storage.has("key")).toBe(true);

        await storage.remove("key");
        expect(await storage.get("key")).toBeNull();
        expect(await storage.has("key")).toBe(false);
      });

      it("should list keys with prefix identically", async () => {
        await storage.set("ns:a", "1");
        await storage.set("ns:b", "2");
        await storage.set("other:c", "3");

        const nsKeys = await storage.keys("ns:");
        expect(nsKeys).toHaveLength(2);
        // Both should contain entries starting with the prefix
        for (const k of nsKeys) {
          expect(k.startsWith("ns:") || k.startsWith("ns_")).toBe(true);
        }
      });

      it("should handle missing keys identically", async () => {
        expect(await storage.get("nonexistent")).toBeNull();
        expect(await storage.has("nonexistent")).toBe(false);
        await expect(storage.remove("nonexistent")).resolves.toBeUndefined();
      });
    });
  }

  runPortContract("FsStorageAdapter", () => {
    const dir = join(TEST_BASE_DIR, `port-fs-${Date.now()}-${Math.random()}`);
    return new FsStorageAdapter(dir);
  });

  runPortContract("MemoryStorageAdapter", () => new MemoryStorageAdapter());

  it("should allow data migration between adapters (swap at runtime)", async () => {
    const source: StoragePort = new MemoryStorageAdapter();
    const targetDir = join(TEST_BASE_DIR, `swap-${Date.now()}`);
    const target: StoragePort = new FsStorageAdapter(targetDir);

    // Populate source
    await source.set("config:theme", "dark");
    await source.set("config:lang", "en");
    await source.set("data:cache", "some-cached-content");

    // Copy all data from source to target
    const allKeys = await source.keys();
    for (const key of allKeys) {
      const value = await source.get(key);
      if (value !== null) {
        await target.set(key, value);
      }
    }

    // Verify target has all data
    expect(await target.get("config:theme")).toBe("dark");
    expect(await target.get("config:lang")).toBe("en");
    expect(await target.get("data:cache")).toBe("some-cached-content");
  });
});

// ── Cookie Import with StoragePort ────────────────────────────────────

describe("Cookie import with StoragePort", () => {
  const SAMPLE_COOKIES_TXT = [
    "# Netscape HTTP Cookie File",
    ".example.com\tTRUE\t/\tFALSE\t1735689600\tsession_id\tabc123",
    ".example.com\tTRUE\t/\tTRUE\t1735689600\tsecure_token\txyz789",
    "api.example.com\tFALSE\t/api\tFALSE\t0\tapi_key\tmy-api-key",
    "",
    "# This is a comment line",
    ".other.com\tTRUE\t/\tFALSE\t1735689600\ttracker\ttrack123",
  ].join("\n");

  it("should parse Netscape cookies.txt content correctly", () => {
    const cookies = parseCookiesTxt(SAMPLE_COOKIES_TXT);

    expect(cookies).toHaveLength(4);

    expect(cookies[0]).toMatchObject({
      name: "session_id",
      value: "abc123",
      domain: ".example.com",
      path: "/",
      secure: false,
    });

    expect(cookies[1]).toMatchObject({
      name: "secure_token",
      value: "xyz789",
      domain: ".example.com",
      path: "/",
      secure: true,
    });

    expect(cookies[2]).toMatchObject({
      name: "api_key",
      value: "my-api-key",
      domain: "api.example.com",
      path: "/api",
      secure: false,
    });

    expect(cookies[3]).toMatchObject({
      name: "tracker",
      value: "track123",
      domain: ".other.com",
    });
  });

  it("should parse expires field correctly", () => {
    const cookies = parseCookiesTxt(SAMPLE_COOKIES_TXT);

    expect(cookies[0]!.expires).toBe(1735689600);
    // expires=0 parsed as falsy → undefined
    expect(cookies[2]!.expires).toBeUndefined();
  });

  it("should import cookies from MemoryStorageAdapter", async () => {
    const storage = new MemoryStorageAdapter();
    await storage.set("cookies:example", SAMPLE_COOKIES_TXT);

    const cookies = await importCookiesTxtFromStorage(
      storage,
      "cookies:example",
    );

    expect(cookies).toHaveLength(4);
    expect(cookies[0]!.name).toBe("session_id");
    expect(cookies[1]!.name).toBe("secure_token");
  });

  it("should import cookies from FsStorageAdapter", async () => {
    const dir = join(TEST_BASE_DIR, `cookie-fs-${Date.now()}`);
    const storage = new FsStorageAdapter(dir);
    await storage.set("cookie-file", SAMPLE_COOKIES_TXT);

    const cookies = await importCookiesTxtFromStorage(storage, "cookie-file");

    expect(cookies).toHaveLength(4);
    expect(cookies[0]!.name).toBe("session_id");
    expect(cookies[3]!.name).toBe("tracker");
  });

  it("should return empty array when storage key does not exist", async () => {
    const storage = new MemoryStorageAdapter();

    const cookies = await importCookiesTxtFromStorage(
      storage,
      "nonexistent-key",
    );

    expect(cookies).toEqual([]);
  });

  it("should handle empty cookies.txt content", () => {
    const cookies = parseCookiesTxt("");
    expect(cookies).toEqual([]);
  });

  it("should handle cookies.txt with only comments", () => {
    const content = "# Comment 1\n# Comment 2\n";
    const cookies = parseCookiesTxt(content);
    expect(cookies).toEqual([]);
  });
});
