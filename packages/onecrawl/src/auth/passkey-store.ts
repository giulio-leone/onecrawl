/**
 * Passkey Store - Persistent encrypted storage for WebAuthn/FIDO2 credentials
 *
 * Stores credentials on disk with AES-256-GCM encryption.
 * Key is derived from machine-id via PBKDF2, or from a generated random key.
 * Only Node.js built-in modules are used.
 */

import { randomBytes, pbkdf2, createCipheriv, createDecipheriv } from "crypto";
import { readFile, writeFile, rename, unlink, access, mkdir } from "fs/promises";
import { join, dirname } from "path";
import { homedir, platform } from "os";
import { execSync } from "child_process";

/** WebAuthn credential stored by the passkey manager */
export interface WebAuthnCredential {
  credentialId: string;
  isResidentCredential: boolean;
  rpId: string;
  privateKey: string;
  userHandle: string;
  signCount: number;
  largeBlob?: string;
}

/** Versioned container for persisted passkey data */
export interface PasskeyStoreData {
  version: 1;
  credentials: WebAuthnCredential[];
  metadata: {
    createdAt: string;
    updatedAt: string;
    rpId: string;
  };
}

/** Shape of the encrypted blob written to disk */
interface EncryptedData {
  iv: string;
  salt: string;
  data: string;
  tag: string;
}

interface PasskeyStoreOptions {
  storagePath: string;
  keyPath?: string;
}

const PBKDF2_ITERATIONS = 100_000;
const KEY_LENGTH = 32;
const IV_LENGTH = 16;
const SALT_LENGTH = 32;

function pbkdf2Async(
  password: string,
  salt: Buffer,
  iterations: number,
  keylen: number,
  digest: string,
): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    pbkdf2(password, salt, iterations, keylen, digest, (err, key) => {
      if (err) reject(err);
      else resolve(key);
    });
  });
}

export class PasskeyStore {
  private readonly storagePath: string;
  private readonly keyPath: string;
  private cachedKey: Buffer | null = null;

  constructor(options: PasskeyStoreOptions) {
    this.storagePath = options.storagePath;
    this.keyPath =
      options.keyPath ?? join(homedir(), ".onecrawl", "key");
  }

  /** Load and decrypt passkey data. Returns null if the store does not exist. */
  async load(): Promise<PasskeyStoreData | null> {
    try {
      await access(this.storagePath);
    } catch {
      return null;
    }

    try {
      const raw = await readFile(this.storagePath, "utf-8");
      const encrypted: EncryptedData = JSON.parse(raw);
      const plaintext = await this.decrypt(encrypted);
      return JSON.parse(plaintext) as PasskeyStoreData;
    } catch (err) {
      throw new Error(
        `Failed to load passkey store: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Encrypt and atomically write passkey data to disk. */
  async save(data: PasskeyStoreData): Promise<void> {
    const dir = dirname(this.storagePath);
    await mkdir(dir, { recursive: true });

    const plaintext = JSON.stringify(data);
    const encrypted = await this.encrypt(plaintext);

    const tmpPath = `${this.storagePath}.tmp`;
    await writeFile(tmpPath, JSON.stringify(encrypted), {
      encoding: "utf-8",
      mode: 0o600,
    });
    await rename(tmpPath, this.storagePath);
  }

  /** Add or update a credential for the given rpId. */
  async addCredential(
    credential: WebAuthnCredential,
    rpId: string,
  ): Promise<void> {
    const now = new Date().toISOString();
    let data = await this.load();

    if (!data) {
      data = {
        version: 1,
        credentials: [],
        metadata: { createdAt: now, updatedAt: now, rpId },
      };
    }

    const idx = data.credentials.findIndex(
      (c) => c.credentialId === credential.credentialId,
    );
    if (idx >= 0) {
      data.credentials[idx] = credential;
    } else {
      data.credentials.push(credential);
    }

    data.metadata.updatedAt = now;
    await this.save(data);
  }

  /** Retrieve credentials, optionally filtered by rpId. */
  async getCredentials(rpId?: string): Promise<WebAuthnCredential[]> {
    const data = await this.load();
    if (!data) return [];

    if (rpId) {
      return data.credentials.filter((c) => c.rpId === rpId);
    }
    return data.credentials;
  }

  /** Remove a credential by its id. */
  async removeCredential(credentialId: string): Promise<void> {
    const data = await this.load();
    if (!data) return;

    data.credentials = data.credentials.filter(
      (c) => c.credentialId !== credentialId,
    );
    data.metadata.updatedAt = new Date().toISOString();
    await this.save(data);
  }

  /** Delete the passkey store file. */
  async clear(): Promise<void> {
    try {
      await unlink(this.storagePath);
    } catch (err: unknown) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    }
  }

  /** Check whether the passkey store file exists and is readable. */
  async exists(): Promise<boolean> {
    try {
      await access(this.storagePath);
      return true;
    } catch {
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private async getMachineId(): Promise<string | null> {
    try {
      if (platform() === "linux") {
        const id = await readFile("/etc/machine-id", "utf-8");
        return id.trim() || null;
      }

      if (platform() === "darwin") {
        const raw = execSync(
          "ioreg -rd1 -c IOPlatformExpertDevice | grep IOPlatformUUID",
          { encoding: "utf-8", timeout: 5_000 },
        );
        const match = raw.match(/"IOPlatformUUID"\s*=\s*"([^"]+)"/);
        return match?.[1] ?? null;
      }
    } catch {
      // Machine-id unavailable — fall through to generated key
    }

    return null;
  }

  private async getOrCreateKey(): Promise<string> {
    if (this.cachedKey) return this.cachedKey.toString("hex");

    const machineId = await this.getMachineId();
    if (machineId) return machineId;

    // Fall back to a generated key stored on disk
    try {
      const stored = await readFile(this.keyPath, "utf-8");
      const trimmed = stored.trim();
      if (trimmed) return trimmed;
    } catch {
      // Key file doesn't exist yet
    }

    const generated = randomBytes(32).toString("hex");
    const keyDir = dirname(this.keyPath);
    await mkdir(keyDir, { recursive: true });
    await writeFile(this.keyPath, generated, {
      encoding: "utf-8",
      mode: 0o600,
    });

    return generated;
  }

  private async encrypt(plaintext: string): Promise<EncryptedData> {
    const secret = await this.getOrCreateKey();
    const salt = randomBytes(SALT_LENGTH);
    const iv = randomBytes(IV_LENGTH);
    const key = await pbkdf2Async(
      secret,
      salt,
      PBKDF2_ITERATIONS,
      KEY_LENGTH,
      "sha512",
    );

    const cipher = createCipheriv("aes-256-gcm", key, iv);
    const encrypted = Buffer.concat([
      cipher.update(plaintext, "utf-8"),
      cipher.final(),
    ]);
    const tag = cipher.getAuthTag();

    return {
      iv: iv.toString("hex"),
      salt: salt.toString("hex"),
      data: encrypted.toString("hex"),
      tag: tag.toString("hex"),
    };
  }

  private async decrypt(encrypted: EncryptedData): Promise<string> {
    const secret = await this.getOrCreateKey();
    const salt = Buffer.from(encrypted.salt, "hex");
    const iv = Buffer.from(encrypted.iv, "hex");
    const tag = Buffer.from(encrypted.tag, "hex");
    const data = Buffer.from(encrypted.data, "hex");

    const key = await pbkdf2Async(
      secret,
      salt,
      PBKDF2_ITERATIONS,
      KEY_LENGTH,
      "sha512",
    );

    const decipher = createDecipheriv("aes-256-gcm", key, iv);
    decipher.setAuthTag(tag);

    try {
      const decrypted = Buffer.concat([
        decipher.update(data),
        decipher.final(),
      ]);
      return decrypted.toString("utf-8");
    } catch {
      throw new Error("Decryption failed: data is corrupted or key mismatch");
    }
  }
}
