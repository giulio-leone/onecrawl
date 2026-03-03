/**
 * OAuth 2.1 module unit tests (M13)
 */

import { describe, it, expect, afterEach, vi, beforeEach } from "vitest";
import { mkdtemp, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { createHash } from "crypto";

import {
  generateCodeVerifier,
  generateCodeChallenge,
  generateState,
} from "../../src/auth/oauth-pkce.js";
import { OAuthTokenStore } from "../../src/auth/oauth-token-store.js";
import { LinkedInOAuthAdapter } from "../../src/adapters/oauth/linkedin-oauth.adapter.js";
import { OAuthRefreshManager } from "../../src/auth/oauth-refresh.js";
import type { OAuthTokens } from "../../src/ports/oauth.port.js";

// =============================================================================
// PKCE
// =============================================================================

describe("PKCE utilities", () => {
  it("generateCodeVerifier returns URL-safe string of correct length", () => {
    const v = generateCodeVerifier();
    expect(v).toHaveLength(64);
    expect(v).toMatch(/^[A-Za-z0-9\-._~]+$/);
  });

  it("generateCodeVerifier respects custom length", () => {
    expect(generateCodeVerifier(43)).toHaveLength(43);
    expect(generateCodeVerifier(128)).toHaveLength(128);
  });

  it("generateCodeVerifier rejects out-of-range length", () => {
    expect(() => generateCodeVerifier(42)).toThrow(RangeError);
    expect(() => generateCodeVerifier(129)).toThrow(RangeError);
  });

  it("generateCodeChallenge produces deterministic S256 output", () => {
    const verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const expected = createHash("sha256")
      .update(verifier, "ascii")
      .digest("base64url");

    expect(generateCodeChallenge(verifier)).toBe(expected);
  });

  it("generateCodeChallenge differs for different verifiers", () => {
    const a = generateCodeChallenge("verifier-one");
    const b = generateCodeChallenge("verifier-two");
    expect(a).not.toBe(b);
  });

  it("generateState returns base64url string", () => {
    const s = generateState();
    expect(s.length).toBeGreaterThanOrEqual(32);
    expect(s).toMatch(/^[A-Za-z0-9\-_]+$/);
  });
});

// =============================================================================
// Token Store
// =============================================================================

describe("OAuthTokenStore", () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "oauth-token-test-"));
  });

  afterEach(async () => {
    if (tmpDir) {
      await rm(tmpDir, { recursive: true, force: true });
    }
  });

  function makeStore() {
    return new OAuthTokenStore({
      storagePath: join(tmpDir, "tokens.json"),
      keyPath: join(tmpDir, "key"),
    });
  }

  function sampleTokens(overrides: Partial<OAuthTokens> = {}): OAuthTokens {
    return {
      accessToken: "acc_test_123",
      refreshToken: "ref_test_456",
      expiresAt: Date.now() + 3_600_000,
      tokenType: "Bearer",
      scope: "openid profile email",
      ...overrides,
    };
  }

  it("returns null when no tokens stored", async () => {
    const store = makeStore();
    expect(await store.getTokens()).toBeNull();
  });

  it("save and load round-trip preserves data", async () => {
    const store = makeStore();
    const tokens = sampleTokens();

    await store.saveTokens(tokens);
    const loaded = await store.getTokens();

    expect(loaded).toEqual(tokens);
  });

  it("clearTokens removes stored data", async () => {
    const store = makeStore();
    await store.saveTokens(sampleTokens());
    await store.clearTokens();
    expect(await store.getTokens()).toBeNull();
  });

  it("clearTokens is idempotent (no error on missing file)", async () => {
    const store = makeStore();
    await expect(store.clearTokens()).resolves.not.toThrow();
  });

  it("isExpired returns true when no tokens", async () => {
    const store = makeStore();
    expect(await store.isExpired()).toBe(true);
  });

  it("isExpired returns false for future token", async () => {
    const store = makeStore();
    await store.saveTokens(sampleTokens({ expiresAt: Date.now() + 60_000 }));
    expect(await store.isExpired()).toBe(false);
  });

  it("isExpired returns true for past token", async () => {
    const store = makeStore();
    await store.saveTokens(sampleTokens({ expiresAt: Date.now() - 1 }));
    expect(await store.isExpired()).toBe(true);
  });
});

// =============================================================================
// LinkedIn OAuth Adapter
// =============================================================================

describe("LinkedInOAuthAdapter", () => {
  const adapter = new LinkedInOAuthAdapter({
    clientId: "test-client-id",
    redirectUri: "http://localhost:3000/callback",
  });

  it("builds correct authorization URL", () => {
    const url = adapter.getAuthorizationUrl("state123", "challenge456");
    const parsed = new URL(url);

    expect(parsed.origin + parsed.pathname).toBe(
      "https://www.linkedin.com/oauth/v2/authorization",
    );
    expect(parsed.searchParams.get("response_type")).toBe("code");
    expect(parsed.searchParams.get("client_id")).toBe("test-client-id");
    expect(parsed.searchParams.get("redirect_uri")).toBe(
      "http://localhost:3000/callback",
    );
    expect(parsed.searchParams.get("state")).toBe("state123");
    expect(parsed.searchParams.get("code_challenge")).toBe("challenge456");
    expect(parsed.searchParams.get("code_challenge_method")).toBe("S256");
    expect(parsed.searchParams.get("scope")).toBe("openid profile email");
  });

  it("includes custom scopes when provided", () => {
    const custom = new LinkedInOAuthAdapter({
      clientId: "cid",
      redirectUri: "http://localhost/cb",
      scopes: ["r_liteprofile", "w_member_social"],
    });
    const url = custom.getAuthorizationUrl("s", "c");
    const parsed = new URL(url);
    expect(parsed.searchParams.get("scope")).toBe(
      "r_liteprofile w_member_social",
    );
  });

  it("exchangeCode calls token endpoint with correct params", async () => {
    const mockResponse = {
      access_token: "new_access",
      refresh_token: "new_refresh",
      expires_in: 3600,
      token_type: "Bearer",
      scope: "openid",
    };

    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      new Response(JSON.stringify(mockResponse), { status: 200 }),
    );

    const tokens = await adapter.exchangeCode("auth_code", "verifier123");

    expect(fetchSpy).toHaveBeenCalledOnce();
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe("https://www.linkedin.com/oauth/v2/accessToken");
    expect(init?.method).toBe("POST");

    const body = new URLSearchParams(init?.body as string);
    expect(body.get("grant_type")).toBe("authorization_code");
    expect(body.get("code")).toBe("auth_code");
    expect(body.get("code_verifier")).toBe("verifier123");

    expect(tokens.accessToken).toBe("new_access");
    expect(tokens.refreshToken).toBe("new_refresh");
    expect(tokens.tokenType).toBe("Bearer");

    fetchSpy.mockRestore();
  });

  it("exchangeCode throws on HTTP error", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      new Response("bad request", { status: 400 }),
    );

    await expect(adapter.exchangeCode("bad", "bad")).rejects.toThrow(
      /Token exchange failed \(400\)/,
    );

    fetchSpy.mockRestore();
  });

  it("refreshToken preserves original refresh token when not returned", async () => {
    const mockResponse = {
      access_token: "refreshed",
      expires_in: 7200,
      token_type: "Bearer",
    };

    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      new Response(JSON.stringify(mockResponse), { status: 200 }),
    );

    const tokens = await adapter.refreshToken("original_rt");
    expect(tokens.refreshToken).toBe("original_rt");

    fetchSpy.mockRestore();
  });
});

// =============================================================================
// Refresh Manager
// =============================================================================

describe("OAuthRefreshManager", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("triggers refresh when token is near expiry", async () => {
    const nearExpiry: OAuthTokens = {
      accessToken: "old",
      refreshToken: "rt",
      expiresAt: Date.now() + 2 * 60_000, // 2 min left (< 5 min buffer)
      tokenType: "Bearer",
    };

    const refreshed: OAuthTokens = {
      accessToken: "new",
      refreshToken: "rt2",
      expiresAt: Date.now() + 3_600_000,
      tokenType: "Bearer",
    };

    const mockAdapter = {
      getAuthorizationUrl: vi.fn(),
      exchangeCode: vi.fn(),
      refreshToken: vi.fn().mockResolvedValue(refreshed),
      revokeToken: vi.fn(),
    };

    const mockStore = {
      getTokens: vi.fn().mockResolvedValue(nearExpiry),
      saveTokens: vi.fn().mockResolvedValue(undefined),
    } as unknown as OAuthTokenStore;

    const manager = new OAuthRefreshManager();
    const onSuccess = vi.fn();
    manager.on("refresh_success", onSuccess);

    manager.startRefreshLoop(mockAdapter, mockStore, 1_000);

    await vi.advanceTimersByTimeAsync(1_000);

    expect(mockAdapter.refreshToken).toHaveBeenCalledWith("rt");
    expect(mockStore.saveTokens).toHaveBeenCalledWith(refreshed);
    expect(onSuccess).toHaveBeenCalledWith({ expiresAt: refreshed.expiresAt });

    manager.stopRefreshLoop();
  });

  it("does not refresh when token has plenty of time", async () => {
    const longLived: OAuthTokens = {
      accessToken: "ok",
      refreshToken: "rt",
      expiresAt: Date.now() + 60 * 60_000, // 60 min left
      tokenType: "Bearer",
    };

    const mockAdapter = {
      getAuthorizationUrl: vi.fn(),
      exchangeCode: vi.fn(),
      refreshToken: vi.fn(),
      revokeToken: vi.fn(),
    };

    const mockStore = {
      getTokens: vi.fn().mockResolvedValue(longLived),
      saveTokens: vi.fn(),
    } as unknown as OAuthTokenStore;

    const manager = new OAuthRefreshManager();
    manager.startRefreshLoop(mockAdapter, mockStore, 1_000);

    await vi.advanceTimersByTimeAsync(1_000);

    expect(mockAdapter.refreshToken).not.toHaveBeenCalled();

    manager.stopRefreshLoop();
  });

  it("emits refresh_error on failure", async () => {
    const nearExpiry: OAuthTokens = {
      accessToken: "old",
      refreshToken: "rt",
      expiresAt: Date.now() + 1_000,
      tokenType: "Bearer",
    };

    const mockAdapter = {
      getAuthorizationUrl: vi.fn(),
      exchangeCode: vi.fn(),
      refreshToken: vi.fn().mockRejectedValue(new Error("network error")),
      revokeToken: vi.fn(),
    };

    const mockStore = {
      getTokens: vi.fn().mockResolvedValue(nearExpiry),
      saveTokens: vi.fn(),
    } as unknown as OAuthTokenStore;

    const manager = new OAuthRefreshManager();
    const onError = vi.fn();
    manager.on("refresh_error", onError);

    manager.startRefreshLoop(mockAdapter, mockStore, 1_000);

    await vi.advanceTimersByTimeAsync(1_000);

    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ message: "network error" }));

    manager.stopRefreshLoop();
  });

  it("stopRefreshLoop clears the interval", () => {
    const manager = new OAuthRefreshManager();
    const mockAdapter = {
      getAuthorizationUrl: vi.fn(),
      exchangeCode: vi.fn(),
      refreshToken: vi.fn(),
      revokeToken: vi.fn(),
    };
    const mockStore = {
      getTokens: vi.fn(),
      saveTokens: vi.fn(),
    } as unknown as OAuthTokenStore;

    manager.startRefreshLoop(mockAdapter, mockStore, 500);
    manager.stopRefreshLoop();

    // Advance time — should not trigger any calls
    vi.advanceTimersByTime(5_000);
    expect(mockStore.getTokens).not.toHaveBeenCalled();
  });
});
