/**
 * AuthCascade unit tests (M1-I8)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdtemp, rm, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

import { AuthCascade, type AuthCascadeOptions } from "../../src/auth/auth-cascade.js";
import type { CDPSession } from "../../src/auth/webauthn-manager.js";
import type { Cookie } from "../../src/auth/cookies.js";

// ── helpers ──────────────────────────────────────────────────────────────────

function createMockCDP(): CDPSession {
  return {
    send: vi.fn<CDPSession["send"]>().mockResolvedValue(undefined),
  };
}

function makeCookie(name = "li_at", value = "token-123"): Cookie {
  return { name, value, domain: ".linkedin.com", path: "/" };
}

describe("AuthCascade", () => {
  let tmpDir: string;
  let cdp: CDPSession;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "auth-cascade-test-"));
    cdp = createMockCDP();
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  function makeOptions(overrides: Partial<AuthCascadeOptions> = {}): AuthCascadeOptions {
    return {
      passkeyStorePath: join(tmpDir, "passkeys.json"),
      cookiePath: join(tmpDir, "cookies.json"),
      rpId: "www.linkedin.com",
      ...overrides,
    };
  }

  // ── Auto mode ───────────────────────────────────────────────────────────

  describe("auto mode", () => {
    it("passkey succeeds → returns passkey result", async () => {
      // Set up a valid passkey store by using PasskeyStore directly
      const { PasskeyStore } = await import("../../src/auth/passkey-store.js");
      const store = new PasskeyStore({
        storagePath: join(tmpDir, "passkeys.json"),
        keyPath: join(tmpDir, "key"),
      });
      await store.addCredential(
        {
          credentialId: "cred-1",
          isResidentCredential: true,
          rpId: "www.linkedin.com",
          privateKey: "pk",
          userHandle: "uh",
          signCount: 0,
        },
        "www.linkedin.com",
      );

      // Mock CDP to return authenticatorId
      (cdp.send as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(undefined) // WebAuthn.enable
        .mockResolvedValueOnce({ authenticatorId: "auth-1" }) // addVirtualAuthenticator
        .mockResolvedValueOnce(undefined) // addCredential (from injectCredentials → setupForPasskeys already called)
        .mockResolvedValue(undefined);

      const cascade = new AuthCascade(makeOptions({ method: "auto" }));
      const result = await cascade.authenticate(cdp);
      expect(result.method).toBe("passkey");
      expect(result.success).toBe(true);
    });

    it("passkey fails → tries cookie → succeeds with cookies", async () => {
      // No passkey store on disk — passkey will fail
      const cookies = [makeCookie()];
      await writeFile(join(tmpDir, "cookies.json"), JSON.stringify(cookies), "utf-8");

      const cascade = new AuthCascade(makeOptions({ method: "auto" }));
      const result = await cascade.authenticate(cdp);
      expect(result.method).toBe("cookie");
      expect(result.success).toBe(true);
      expect(result.cookies).toHaveLength(1);
    });

    it("both fail → calls onManualLoginRequired", async () => {
      const manual = vi.fn().mockResolvedValue(undefined);

      const cascade = new AuthCascade(
        makeOptions({ method: "auto", onManualLoginRequired: manual }),
      );
      const result = await cascade.authenticate(cdp);

      expect(manual).toHaveBeenCalled();
      expect(result.method).toBe("auto");
      expect(result.success).toBe(true);
    });

    it("both fail, no callback → returns failure", async () => {
      const cascade = new AuthCascade(makeOptions({ method: "auto" }));
      const result = await cascade.authenticate(cdp);

      expect(result.success).toBe(false);
      expect(result.error).toContain("No passkey, OAuth, or cookie credentials available");
    });
  });

  // ── Passkey mode ────────────────────────────────────────────────────────

  describe("passkey mode", () => {
    it("succeeds normally", async () => {
      const { PasskeyStore } = await import("../../src/auth/passkey-store.js");
      const store = new PasskeyStore({
        storagePath: join(tmpDir, "passkeys.json"),
        keyPath: join(tmpDir, "key"),
      });
      await store.addCredential(
        {
          credentialId: "cred-pk",
          isResidentCredential: true,
          rpId: "www.linkedin.com",
          privateKey: "pk",
          userHandle: "uh",
          signCount: 0,
        },
        "www.linkedin.com",
      );

      (cdp.send as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(undefined) // enable
        .mockResolvedValueOnce({ authenticatorId: "auth-pk" }) // addVirtualAuthenticator
        .mockResolvedValue(undefined); // addCredential

      const cascade = new AuthCascade(makeOptions({ method: "passkey" }));
      const result = await cascade.authenticate(cdp);
      expect(result.method).toBe("passkey");
      expect(result.success).toBe(true);
    });

    it("fails with error when no credentials", async () => {
      const cascade = new AuthCascade(makeOptions({ method: "passkey" }));
      const result = await cascade.authenticate(cdp);
      expect(result.success).toBe(false);
      expect(result.error).toContain("No passkey credentials found");
    });
  });

  // ── Cookie mode ─────────────────────────────────────────────────────────

  describe("cookie mode", () => {
    it("succeeds with cookies array", async () => {
      const cookies = [makeCookie(), makeCookie("JSESSIONID", "sess-abc")];
      await writeFile(join(tmpDir, "cookies.json"), JSON.stringify(cookies), "utf-8");

      const cascade = new AuthCascade(makeOptions({ method: "cookie" }));
      const result = await cascade.authenticate(cdp);
      expect(result.method).toBe("cookie");
      expect(result.success).toBe(true);
      expect(result.cookies).toHaveLength(2);
    });

    it("fails when no cookie file", async () => {
      const cascade = new AuthCascade(makeOptions({ method: "cookie" }));
      const result = await cascade.authenticate(cdp);
      expect(result.success).toBe(false);
      expect(result.error).toContain("Cookie auth failed");
    });
  });

  // ── getStatus ───────────────────────────────────────────────────────────

  describe("getStatus()", () => {
    it("reports correctly: no auth available", async () => {
      const cascade = new AuthCascade(makeOptions());
      const status = await cascade.getStatus();
      expect(status.passkey).toBe(false);
      expect(status.cookie).toBe(false);
    });

    it("reports correctly: only cookies", async () => {
      const cookies = [makeCookie()];
      await writeFile(join(tmpDir, "cookies.json"), JSON.stringify(cookies), "utf-8");

      const cascade = new AuthCascade(makeOptions());
      const status = await cascade.getStatus();
      expect(status.passkey).toBe(false);
      expect(status.cookie).toBe(true);
      expect(status.cookieCount).toBe(1);
    });

    it("reports correctly: only passkey", async () => {
      const { PasskeyStore } = await import("../../src/auth/passkey-store.js");
      const store = new PasskeyStore({
        storagePath: join(tmpDir, "passkeys.json"),
        keyPath: join(tmpDir, "key"),
      });
      await store.addCredential(
        {
          credentialId: "cred-s",
          isResidentCredential: true,
          rpId: "www.linkedin.com",
          privateKey: "pk",
          userHandle: "uh",
          signCount: 0,
        },
        "www.linkedin.com",
      );

      const cascade = new AuthCascade(makeOptions());
      const status = await cascade.getStatus();
      expect(status.passkey).toBe(true);
      expect(status.passkeyRpId).toBe("www.linkedin.com");
      expect(status.cookie).toBe(false);
    });

    it("reports correctly: both available", async () => {
      const { PasskeyStore } = await import("../../src/auth/passkey-store.js");
      const store = new PasskeyStore({
        storagePath: join(tmpDir, "passkeys.json"),
        keyPath: join(tmpDir, "key"),
      });
      await store.addCredential(
        {
          credentialId: "cred-b",
          isResidentCredential: true,
          rpId: "www.linkedin.com",
          privateKey: "pk",
          userHandle: "uh",
          signCount: 0,
        },
        "www.linkedin.com",
      );
      await writeFile(
        join(tmpDir, "cookies.json"),
        JSON.stringify([makeCookie()]),
        "utf-8",
      );

      const cascade = new AuthCascade(makeOptions());
      const status = await cascade.getStatus();
      expect(status.passkey).toBe(true);
      expect(status.cookie).toBe(true);
    });
  });

  // ── getLoadedCookies ────────────────────────────────────────────────────

  describe("getLoadedCookies()", () => {
    it("returns null before auth", async () => {
      const cascade = new AuthCascade(makeOptions());
      expect(cascade.getLoadedCookies()).toBeNull();
    });

    it("returns cookies after cookie auth", async () => {
      const cookies = [makeCookie()];
      await writeFile(join(tmpDir, "cookies.json"), JSON.stringify(cookies), "utf-8");

      const cascade = new AuthCascade(makeOptions({ method: "cookie" }));
      await cascade.authenticate(cdp);

      const loaded = cascade.getLoadedCookies();
      expect(loaded).not.toBeNull();
      expect(loaded).toHaveLength(1);
      expect(loaded![0].name).toBe("li_at");
    });
  });
});
