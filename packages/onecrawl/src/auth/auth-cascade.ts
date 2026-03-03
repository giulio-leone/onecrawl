/**
 * Auth Cascade — tries passkey auth first, falls back to cookies, then prompts
 * for manual login. Designed for headless LinkedIn automation where multiple
 * auth strategies must be attempted in priority order.
 */

import { readFile, access } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

import { PasskeyStore } from "./passkey-store.js";
import { WebAuthnManager, type CDPSession } from "./webauthn-manager.js";
import type { Cookie } from "./cookies.js";

// ── Types ────────────────────────────────────────────────────────────────────

export type AuthMethod = "auto" | "passkey" | "cookie";

export type AuthResult = {
  method: AuthMethod;
  success: boolean;
  error?: string;
  /** Populated when method is 'cookie' and auth succeeds. */
  cookies?: Cookie[];
};

export interface AuthCascadeOptions {
  /** Auth strategy. @default 'auto' */
  method?: AuthMethod;
  /** Path to the encrypted passkey store. @default ~/.onecrawl/linkedin/passkey.json */
  passkeyStorePath?: string;
  /** Path to the Playwright-format cookie JSON file. @default ~/.onecrawl/linkedin/cookies.json */
  cookiePath?: string;
  /** Relying-party ID for WebAuthn. @default 'www.linkedin.com' */
  rpId?: string;
  /** Callback invoked when manual login is the only option left. */
  onManualLoginRequired?: () => Promise<void>;
  /** Timeout in ms for individual auth attempts. @default 30000 */
  timeout?: number;
}

// ── Defaults ─────────────────────────────────────────────────────────────────

const DEFAULT_PASSKEY_PATH = join(
  homedir(),
  ".onecrawl",
  "linkedin",
  "passkey.json",
);
const DEFAULT_COOKIE_PATH = join(
  homedir(),
  ".onecrawl",
  "linkedin",
  "cookies.json",
);
const DEFAULT_RP_ID = "www.linkedin.com";
const DEFAULT_TIMEOUT = 30_000;

// ── AuthCascade ──────────────────────────────────────────────────────────────

export class AuthCascade {
  private readonly method: AuthMethod;
  private readonly passkeyStorePath: string;
  private readonly cookiePath: string;
  private readonly rpId: string;
  private readonly onManualLoginRequired?: () => Promise<void>;
  private readonly timeout: number;

  private passkeyStore: PasskeyStore | null = null;
  private loadedCookies: Cookie[] | null = null;

  constructor(options: AuthCascadeOptions = {}) {
    this.method = options.method ?? "auto";
    this.passkeyStorePath = options.passkeyStorePath ?? DEFAULT_PASSKEY_PATH;
    this.cookiePath = options.cookiePath ?? DEFAULT_COOKIE_PATH;
    this.rpId = options.rpId ?? DEFAULT_RP_ID;
    this.onManualLoginRequired = options.onManualLoginRequired;
    this.timeout = options.timeout ?? DEFAULT_TIMEOUT;
  }

  /**
   * Run the authentication cascade.
   *
   * - `auto`: passkey → cookie → manual login
   * - `passkey`: passkey only, fails if unavailable
   * - `cookie`: cookie only
   */
  async authenticate(cdpSession: CDPSession): Promise<AuthResult> {
    switch (this.method) {
      case "passkey":
        return this.tryPasskeyAuth(cdpSession);

      case "cookie":
        return this.tryCookieAuth();

      case "auto": {
        const passkeyResult = await this.tryPasskeyAuth(cdpSession);
        if (passkeyResult.success) return passkeyResult;

        const cookieResult = await this.tryCookieAuth();
        if (cookieResult.success) return cookieResult;

        return this.requestManualLogin();
      }
    }
  }

  /**
   * Check which auth methods are currently available on disk.
   */
  async getStatus(): Promise<{
    passkey: boolean;
    cookie: boolean;
    passkeyRpId?: string;
    cookieCount?: number;
  }> {
    const store = this.getPasskeyStore();
    const credentials = await store.getCredentials(this.rpId);
    const hasPasskey = credentials.length > 0;

    let hasCookie = false;
    let cookieCount: number | undefined;
    try {
      await access(this.cookiePath);
      const raw = await readFile(this.cookiePath, "utf-8");
      const parsed: unknown = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        hasCookie = parsed.length > 0;
        cookieCount = parsed.length;
      }
    } catch {
      // Cookie file missing or unreadable
    }

    return {
      passkey: hasPasskey,
      cookie: hasCookie,
      ...(hasPasskey ? { passkeyRpId: this.rpId } : {}),
      ...(cookieCount !== undefined ? { cookieCount } : {}),
    };
  }

  /** Return cookies loaded by the last successful `tryCookieAuth` call. */
  getLoadedCookies(): Cookie[] | null {
    return this.loadedCookies;
  }

  // ── Private auth strategies ────────────────────────────────────────────────

  private async tryPasskeyAuth(cdpSession: CDPSession): Promise<AuthResult> {
    try {
      const store = this.getPasskeyStore();
      const credentials = await store.getCredentials(this.rpId);

      if (credentials.length === 0) {
        return {
          method: "passkey",
          success: false,
          error: `No passkey credentials found for rpId "${this.rpId}"`,
        };
      }

      const manager = new WebAuthnManager(cdpSession);
      await manager.setupForPasskeys();
      await manager.injectCredentials(credentials);

      return { method: "passkey", success: true };
    } catch (err) {
      return {
        method: "passkey",
        success: false,
        error: `Passkey auth failed: ${err instanceof Error ? err.message : String(err)}`,
      };
    }
  }

  private async tryCookieAuth(): Promise<AuthResult> {
    try {
      await access(this.cookiePath);

      const raw = await readFile(this.cookiePath, "utf-8");
      const parsed: unknown = JSON.parse(raw);

      if (!Array.isArray(parsed) || parsed.length === 0) {
        return {
          method: "cookie",
          success: false,
          error: "Cookie file is empty or not a JSON array",
        };
      }

      const cookies = parsed as Cookie[];
      this.loadedCookies = cookies;

      return { method: "cookie", success: true, cookies };
    } catch (err) {
      return {
        method: "cookie",
        success: false,
        error: `Cookie auth failed: ${err instanceof Error ? err.message : String(err)}`,
      };
    }
  }

  private async requestManualLogin(): Promise<AuthResult> {
    if (this.onManualLoginRequired) {
      await this.onManualLoginRequired();
      return { method: "auto", success: true };
    }

    return {
      method: "auto",
      success: false,
      error:
        "No passkey or cookie credentials available and no onManualLoginRequired callback provided",
    };
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  private getPasskeyStore(): PasskeyStore {
    if (!this.passkeyStore) {
      this.passkeyStore = new PasskeyStore({
        storagePath: this.passkeyStorePath,
      });
    }
    return this.passkeyStore;
  }
}
