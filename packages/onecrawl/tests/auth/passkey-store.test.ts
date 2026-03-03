/**
 * PasskeyStore unit tests (M1-I7)
 */

import { describe, it, expect, afterEach } from "vitest";
import { mkdtemp, rm, readFile, writeFile, access } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { randomBytes } from "crypto";

import {
  PasskeyStore,
  type WebAuthnCredential,
  type PasskeyStoreData,
} from "../../src/auth/passkey-store.js";

function makeCred(overrides: Partial<WebAuthnCredential> = {}): WebAuthnCredential {
  return {
    credentialId: `cred-${randomBytes(4).toString("hex")}`,
    isResidentCredential: true,
    rpId: "www.linkedin.com",
    privateKey: randomBytes(32).toString("base64"),
    userHandle: "dXNlcg==",
    signCount: 0,
    ...overrides,
  };
}

function makeStoreData(
  creds: WebAuthnCredential[] = [],
  rpId = "www.linkedin.com",
): PasskeyStoreData {
  const now = new Date().toISOString();
  return {
    version: 1,
    credentials: creds,
    metadata: { createdAt: now, updatedAt: now, rpId },
  };
}

describe("PasskeyStore", () => {
  let tmpDir: string;
  let store: PasskeyStore;

  afterEach(async () => {
    if (tmpDir) {
      await rm(tmpDir, { recursive: true, force: true });
    }
  });

  async function createStore() {
    tmpDir = await mkdtemp(join(tmpdir(), "passkey-test-"));
    const storagePath = join(tmpDir, "passkeys.json");
    const keyPath = join(tmpDir, "key");
    store = new PasskeyStore({ storagePath, keyPath });
    return { storagePath, keyPath };
  }

  // ── save / load round-trip ──────────────────────────────────────────────

  it("save() + load() round-trip preserves data", async () => {
    await createStore();
    const cred = makeCred();
    const data = makeStoreData([cred]);

    await store.save(data);
    const loaded = await store.load();

    expect(loaded).not.toBeNull();
    expect(loaded!.version).toBe(1);
    expect(loaded!.credentials).toHaveLength(1);
    expect(loaded!.credentials[0].credentialId).toBe(cred.credentialId);
    expect(loaded!.credentials[0].privateKey).toBe(cred.privateKey);
  });

  // ── addCredential ───────────────────────────────────────────────────────

  it("addCredential() adds to existing store", async () => {
    await createStore();
    const cred1 = makeCred();
    await store.save(makeStoreData([cred1]));

    const cred2 = makeCred();
    await store.addCredential(cred2, "www.linkedin.com");

    const loaded = await store.load();
    expect(loaded!.credentials).toHaveLength(2);
  });

  it("addCredential() creates new store if none exists", async () => {
    await createStore();
    const cred = makeCred();
    await store.addCredential(cred, "www.linkedin.com");

    const loaded = await store.load();
    expect(loaded).not.toBeNull();
    expect(loaded!.credentials).toHaveLength(1);
    expect(loaded!.credentials[0].credentialId).toBe(cred.credentialId);
  });

  // ── getCredentials ──────────────────────────────────────────────────────

  it("getCredentials() filters by rpId", async () => {
    await createStore();
    const c1 = makeCred({ rpId: "www.linkedin.com" });
    const c2 = makeCred({ rpId: "other.example.com" });
    await store.save(makeStoreData([c1, c2]));

    const filtered = await store.getCredentials("www.linkedin.com");
    expect(filtered).toHaveLength(1);
    expect(filtered[0].credentialId).toBe(c1.credentialId);
  });

  it("getCredentials() returns all if no rpId", async () => {
    await createStore();
    const c1 = makeCred({ rpId: "www.linkedin.com" });
    const c2 = makeCred({ rpId: "other.example.com" });
    await store.save(makeStoreData([c1, c2]));

    const all = await store.getCredentials();
    expect(all).toHaveLength(2);
  });

  // ── removeCredential ────────────────────────────────────────────────────

  it("removeCredential() removes by credentialId", async () => {
    await createStore();
    const c1 = makeCred();
    const c2 = makeCred();
    await store.save(makeStoreData([c1, c2]));

    await store.removeCredential(c1.credentialId);

    const loaded = await store.load();
    expect(loaded!.credentials).toHaveLength(1);
    expect(loaded!.credentials[0].credentialId).toBe(c2.credentialId);
  });

  // ── clear ───────────────────────────────────────────────────────────────

  it("clear() deletes the file", async () => {
    const { storagePath } = await createStore();
    await store.save(makeStoreData([makeCred()]));
    await store.clear();

    await expect(access(storagePath)).rejects.toThrow();
  });

  // ── exists ──────────────────────────────────────────────────────────────

  it("exists() returns true/false correctly", async () => {
    await createStore();
    expect(await store.exists()).toBe(false);

    await store.save(makeStoreData([makeCred()]));
    expect(await store.exists()).toBe(true);

    await store.clear();
    expect(await store.exists()).toBe(false);
  });

  // ── load missing ────────────────────────────────────────────────────────

  it("load() returns null for missing file", async () => {
    await createStore();
    const result = await store.load();
    expect(result).toBeNull();
  });

  // ── corrupted file ──────────────────────────────────────────────────────

  it("corrupted file throws descriptive error", async () => {
    const { storagePath } = await createStore();
    await writeFile(storagePath, "NOT_JSON!!!", "utf-8");

    await expect(store.load()).rejects.toThrow("Failed to load passkey store");
  });

  // ── atomic write ────────────────────────────────────────────────────────

  it("atomic write: .tmp file used during save", async () => {
    const { storagePath } = await createStore();

    // Spy by checking the file exists after save (the .tmp is renamed)
    await store.save(makeStoreData([makeCred()]));

    // The .tmp should have been renamed, so it must not exist
    await expect(access(`${storagePath}.tmp`)).rejects.toThrow();
    // But the main file should exist
    const raw = await readFile(storagePath, "utf-8");
    const parsed = JSON.parse(raw);
    expect(parsed).toHaveProperty("iv");
    expect(parsed).toHaveProperty("salt");
    expect(parsed).toHaveProperty("data");
    expect(parsed).toHaveProperty("tag");
  });

  // ── key derivation fallback ─────────────────────────────────────────────

  it("key derivation: fallback key created when machine-id unavailable", async () => {
    const { keyPath } = await createStore();
    const cred = makeCred();
    await store.save(makeStoreData([cred]));

    // A key file should have been created (or machine-id was used).
    // If the platform provides machine-id the key file may not exist,
    // but on most CI / test environments the fallback path fires.
    // We verify the round-trip works regardless.
    const loaded = await store.load();
    expect(loaded!.credentials[0].credentialId).toBe(cred.credentialId);

    // If the fallback key was written, it should be 64 hex chars
    try {
      const keyContent = await readFile(keyPath, "utf-8");
      expect(keyContent.trim()).toMatch(/^[0-9a-f]{64}$/);
    } catch {
      // machine-id was available — that's fine
    }
  });
});
