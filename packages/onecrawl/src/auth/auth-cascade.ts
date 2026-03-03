/**
 * Auth Cascade — tries passkey auth first, falls back to OAuth, cookies, then
 * prompts for manual login. Designed for headless LinkedIn automation where
 * multiple auth strategies must be attempted in priority order.
 */

import { readFile, access } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

import { PasskeyStore } from "./passkey-store.js";
import { WebAuthnManager, type CDPSession } from "./webauthn-manager.js";
import type { Cookie } from "./cookies.js";
import type { OAuthConfig, OAuthTokens } from "../ports/oauth.port.js";
import type { TwoFactorChallenge } from "../ports/twofa.port.js";
import { OAuthTokenStore } from "./oauth-token-store.js";
import { TotpSecretStore } from "./totp-secret-store.js";
import { LinkedInOAuthAdapter } from "../adapters/oauth/linkedin-oauth.adapter.js";
import { generateCodeVerifier, generateCodeChallenge, generateState } from "./oauth-pkce.js";

// ── Types ────────────────────────────────────────────────────────────────────

export type AuthMethod = "auto" | "passkey" | "oauth" | "cookie";

export type AuthResult = {
  method: AuthMethod;
  success: boolean;
  error?: string;
  /** Populated when method is 'cookie' and auth succeeds. */
  cookies?: Cookie[];
  /** Populated when method is 'oauth' and auth succeeds. */
  tokens?: OAuthTokens;
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
  /** OAuth configuration. If provided, enables OAuth auth method. */
  oauthConfig?: OAuthConfig;
  /** Path to encrypted OAuth token store. @default ~/.onecrawl/linkedin/oauth-tokens.json */
  oauthTokenStorePath?: string;
  /** Callback for OAuth authorization URL (user needs to visit and return code). */
  onOAuthRequired?: (authUrl: string) => Promise<string>;
  /** TOTP secret for automated 2FA (base32 encoded). If not provided, uses stored secret. */
  totpSecret?: string;
  /** Path to encrypted TOTP secret store. @default ~/.onecrawl/linkedin/totp-secret.json */
  totpSecretStorePath?: string;
  /** Callback for SMS 2FA code entry. */
  onSmsCodeRequired?: () => Promise<string>;
  /** Callback invoked when 2FA challenge is detected during login. */
  on2faRequired?: (challenge: TwoFactorChallenge) => Promise<string>;
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
const DEFAULT_OAUTH_TOKEN_PATH = join(
  homedir(),
  ".onecrawl",
  "linkedin",
  "oauth-tokens.json",
);
const DEFAULT_TOTP_SECRET_PATH = join(
  homedir(),
  ".onecrawl",
  "linkedin",
  "totp-secret.json",
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
  private readonly oauthConfig?: OAuthConfig;
  private readonly oauthTokenStorePath: string;
  private readonly onOAuthRequired?: (authUrl: string) => Promise<string>;
  private readonly totpSecret?: string;
  private readonly totpSecretStorePath: string;
  private readonly onSmsCodeRequired?: () => Promise<string>;
  private readonly on2faRequired?: (challenge: TwoFactorChallenge) => Promise<string>;

  private passkeyStore: PasskeyStore | null = null;
  private oauthTokenStore: OAuthTokenStore | null = null;
  private totpSecretStore: TotpSecretStore | null = null;
  private loadedCookies: Cookie[] | null = null;

  constructor(options: AuthCascadeOptions = {}) {
    this.method = options.method ?? "auto";
    this.passkeyStorePath = options.passkeyStorePath ?? DEFAULT_PASSKEY_PATH;
    this.cookiePath = options.cookiePath ?? DEFAULT_COOKIE_PATH;
    this.rpId = options.rpId ?? DEFAULT_RP_ID;
    this.onManualLoginRequired = options.onManualLoginRequired;
    this.timeout = options.timeout ?? DEFAULT_TIMEOUT;
    this.oauthConfig = options.oauthConfig;
    this.oauthTokenStorePath = options.oauthTokenStorePath ?? DEFAULT_OAUTH_TOKEN_PATH;
    this.onOAuthRequired = options.onOAuthRequired;
    this.totpSecret = options.totpSecret;
    this.totpSecretStorePath = options.totpSecretStorePath ?? DEFAULT_TOTP_SECRET_PATH;
    this.onSmsCodeRequired = options.onSmsCodeRequired;
    this.on2faRequired = options.on2faRequired;
  }

  /**
   * Run the authentication cascade.
   *
   * - `auto`: passkey → oauth → cookie → manual login
   * - `passkey`: passkey only, fails if unavailable
   * - `oauth`: OAuth 2.1 only
   * - `cookie`: cookie only
   */
  async authenticate(cdpSession: CDPSession): Promise<AuthResult> {
    switch (this.method) {
      case "passkey":
        return this.tryPasskeyAuth(cdpSession);

      case "oauth":
        return this.tryOAuthAuth();

      case "cookie":
        return this.tryCookieAuth();

      case "auto": {
        const passkeyResult = await this.tryPasskeyAuth(cdpSession);
        if (passkeyResult.success) return passkeyResult;

        const oauthResult = await this.tryOAuthAuth();
        if (oauthResult.success) return oauthResult;

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
    oauth: boolean;
    oauthExpiresAt?: number;
    totp: boolean;
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

    let hasOAuth = false;
    let oauthExpiresAt: number | undefined;
    try {
      const tokenStore = this.getOAuthTokenStore();
      const tokens = await tokenStore.getTokens();
      if (tokens) {
        hasOAuth = true;
        oauthExpiresAt = tokens.expiresAt;
      }
    } catch {
      // OAuth tokens missing or unreadable
    }

    const totpStore = this.getTotpSecretStore();
    const hasTotp = this.totpSecret ? true : await totpStore.hasSecret();

    return {
      passkey: hasPasskey,
      cookie: hasCookie,
      oauth: hasOAuth,
      ...(oauthExpiresAt !== undefined ? { oauthExpiresAt } : {}),
      totp: hasTotp,
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
        "No passkey, OAuth, or cookie credentials available and no onManualLoginRequired callback provided",
    };
  }

  private async tryOAuthAuth(): Promise<AuthResult> {
    try {
      const tokenStore = this.getOAuthTokenStore();
      const tokens = await tokenStore.getTokens();

      // 1. Valid non-expired tokens on disk
      if (tokens && Date.now() < tokens.expiresAt) {
        return { method: "oauth", success: true, tokens };
      }

      // 2. Expired but has refresh token → try refresh
      if (tokens?.refreshToken && this.oauthConfig) {
        try {
          const adapter = new LinkedInOAuthAdapter(this.oauthConfig);
          const refreshed = await adapter.refreshToken(tokens.refreshToken);
          await tokenStore.saveTokens(refreshed);
          return { method: "oauth", success: true, tokens: refreshed };
        } catch {
          // Refresh failed — fall through to full auth flow
        }
      }

      // 3. No tokens → initiate OAuth flow via callback
      if (this.onOAuthRequired && this.oauthConfig) {
        const adapter = new LinkedInOAuthAdapter(this.oauthConfig);
        const codeVerifier = generateCodeVerifier();
        const codeChallenge = generateCodeChallenge(codeVerifier);
        const state = generateState();
        const authUrl = adapter.getAuthorizationUrl(state, codeChallenge);

        const code = await this.onOAuthRequired(authUrl);
        const newTokens = await adapter.exchangeCode(code, codeVerifier);
        await tokenStore.saveTokens(newTokens);
        return { method: "oauth", success: true, tokens: newTokens };
      }

      return {
        method: "oauth",
        success: false,
        error: "No OAuth tokens available and no oauthConfig/onOAuthRequired callback provided",
      };
    } catch (err) {
      return {
        method: "oauth",
        success: false,
        error: `OAuth auth failed: ${err instanceof Error ? err.message : String(err)}`,
      };
    }
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

  private getOAuthTokenStore(): OAuthTokenStore {
    if (!this.oauthTokenStore) {
      this.oauthTokenStore = new OAuthTokenStore({
        storagePath: this.oauthTokenStorePath,
      });
    }
    return this.oauthTokenStore;
  }

  private getTotpSecretStore(): TotpSecretStore {
    if (!this.totpSecretStore) {
      this.totpSecretStore = new TotpSecretStore({
        storagePath: this.totpSecretStorePath,
      });
    }
    return this.totpSecretStore;
  }
}
