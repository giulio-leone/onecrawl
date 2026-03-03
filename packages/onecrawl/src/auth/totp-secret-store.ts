/**
 * Encrypted TOTP Secret Store
 *
 * Persists a TOTP shared secret on disk with AES-256-GCM encryption.
 * Key derivation mirrors passkey-store.ts (PBKDF2 from machine-id).
 * Only Node.js built-in modules are used.
 *
 * Default path: ~/.onecrawl/linkedin/totp-secret.json
 */

import { randomBytes, pbkdf2, createCipheriv, createDecipheriv } from "crypto";
import { readFile, writeFile, rename, unlink, access, mkdir } from "fs/promises";
import { join, dirname } from "path";
import { homedir, platform } from "os";
import { execSync } from "child_process";

// =============================================================================
// Types
// =============================================================================

interface EncryptedData {
  iv: string;
  salt: string;
  data: string;
  tag: string;
}

interface TotpSecretData {
  version: 1;
  secret: string;
  metadata: {
    createdAt: string;
    updatedAt: string;
  };
}

export interface TotpSecretStoreOptions {
  /** Path to the encrypted secret file. */
  storagePath?: string;
  /** Path to the fallback encryption key file. */
  keyPath?: string;
}

// =============================================================================
// Constants
// =============================================================================

const PBKDF2_ITERATIONS = 100_000;
const KEY_LENGTH = 32;
const IV_LENGTH = 16;
const SALT_LENGTH = 32;

const DEFAULT_STORAGE_PATH = join(
  homedir(),
  ".onecrawl",
  "linkedin",
  "totp-secret.json",
);

// =============================================================================
// Helpers
// =============================================================================

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

// =============================================================================
// TotpSecretStore
// =============================================================================

export class TotpSecretStore {
  private readonly storagePath: string;
  private readonly keyPath: string;

  constructor(options: TotpSecretStoreOptions = {}) {
    this.storagePath = options.storagePath ?? DEFAULT_STORAGE_PATH;
    this.keyPath = options.keyPath ?? join(homedir(), ".onecrawl", "key");
  }

  /** Encrypt and persist a TOTP secret. */
  async saveSecret(secret: string): Promise<void> {
    const dir = dirname(this.storagePath);
    await mkdir(dir, { recursive: true });

    const now = new Date().toISOString();
    const payload: TotpSecretData = {
      version: 1,
      secret,
      metadata: { createdAt: now, updatedAt: now },
    };

    const encrypted = await this.encrypt(JSON.stringify(payload));
    const tmpPath = `${this.storagePath}.tmp`;
    await writeFile(tmpPath, JSON.stringify(encrypted), {
      encoding: "utf-8",
      mode: 0o600,
    });
    await rename(tmpPath, this.storagePath);
  }

  /** Decrypt and return the stored TOTP secret, or `null` if none exists. */
  async getSecret(): Promise<string | null> {
    try {
      await access(this.storagePath);
    } catch {
      return null;
    }

    try {
      const raw = await readFile(this.storagePath, "utf-8");
      const encrypted: EncryptedData = JSON.parse(raw);
      const plaintext = await this.decrypt(encrypted);
      const data: TotpSecretData = JSON.parse(plaintext);
      return data.secret;
    } catch (err) {
      throw new Error(
        `Failed to load TOTP secret: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Delete the stored secret. */
  async clearSecret(): Promise<void> {
    try {
      await unlink(this.storagePath);
    } catch (err: unknown) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    }
  }

  /** Check whether a stored secret exists. */
  async hasSecret(): Promise<boolean> {
    try {
      await access(this.storagePath);
      return true;
    } catch {
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Private helpers (mirrors passkey-store.ts)
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
    const machineId = await this.getMachineId();
    if (machineId) return machineId;

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
