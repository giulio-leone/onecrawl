/**
 * WebAuthnManager unit tests (M1-I6)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  WebAuthnManager,
  type CDPSession,
  type WebAuthnCredential,
} from "../../src/auth/webauthn-manager.js";

function createMockCDP() {
  return { send: vi.fn<CDPSession["send"]>() } satisfies CDPSession;
}

function makeCred(overrides: Partial<WebAuthnCredential> = {}): WebAuthnCredential {
  return {
    credentialId: "cred-1",
    isResidentCredential: true,
    rpId: "www.linkedin.com",
    privateKey: "pk-base64",
    userHandle: "dXNlcg==",
    signCount: 0,
    ...overrides,
  };
}

describe("WebAuthnManager", () => {
  let cdp: ReturnType<typeof createMockCDP>;
  let mgr: WebAuthnManager;

  beforeEach(() => {
    cdp = createMockCDP();
    cdp.send.mockResolvedValue(undefined);
    mgr = new WebAuthnManager(cdp);
  });

  // ── enable / disable ────────────────────────────────────────────────────

  it("enable() sends WebAuthn.enable with enableUI: false", async () => {
    await mgr.enable();
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.enable", { enableUI: false });
  });

  it("disable() sends WebAuthn.disable", async () => {
    await mgr.disable();
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.disable");
  });

  // ── addAuthenticator ────────────────────────────────────────────────────

  it("addAuthenticator() returns authenticatorId from CDP response", async () => {
    cdp.send.mockResolvedValueOnce({ authenticatorId: "auth-42" });
    const id = await mgr.addAuthenticator();
    expect(id).toBe("auth-42");
    expect(cdp.send).toHaveBeenCalledWith(
      "WebAuthn.addVirtualAuthenticator",
      expect.objectContaining({ options: expect.objectContaining({ protocol: "ctap2" }) }),
    );
  });

  it("addAuthenticator() with custom options passes them correctly", async () => {
    cdp.send.mockResolvedValueOnce({ authenticatorId: "auth-99" });
    const id = await mgr.addAuthenticator({ transport: "usb", hasResidentKey: false });
    expect(id).toBe("auth-99");
    expect(cdp.send).toHaveBeenCalledWith(
      "WebAuthn.addVirtualAuthenticator",
      expect.objectContaining({
        options: expect.objectContaining({ transport: "usb", hasResidentKey: false }),
      }),
    );
  });

  // ── credential CRUD ─────────────────────────────────────────────────────

  it("addCredential() sends correct params", async () => {
    const cred = makeCred();
    await mgr.addCredential("auth-1", cred);
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.addCredential", {
      authenticatorId: "auth-1",
      credential: cred,
    });
  });

  it("getCredentials() returns parsed credentials", async () => {
    const creds = [makeCred(), makeCred({ credentialId: "cred-2" })];
    cdp.send.mockResolvedValueOnce({ credentials: creds });
    const result = await mgr.getCredentials("auth-1");
    expect(result).toEqual(creds);
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.getCredentials", {
      authenticatorId: "auth-1",
    });
  });

  it("removeCredential() sends correct params", async () => {
    await mgr.removeCredential("auth-1", "cred-1");
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.removeCredential", {
      authenticatorId: "auth-1",
      credentialId: "cred-1",
    });
  });

  it("clearCredentials() sends correct params", async () => {
    await mgr.clearCredentials("auth-1");
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.clearCredentials", {
      authenticatorId: "auth-1",
    });
  });

  // ── setUserVerified ─────────────────────────────────────────────────────

  it("setUserVerified() sends correct params", async () => {
    await mgr.setUserVerified("auth-1", true);
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.setUserVerified", {
      authenticatorId: "auth-1",
      isUserVerified: true,
    });
  });

  // ── high-level helpers ──────────────────────────────────────────────────

  it("setupForPasskeys() calls enable + addAuthenticator with defaults", async () => {
    cdp.send
      .mockResolvedValueOnce(undefined) // enable
      .mockResolvedValueOnce({ authenticatorId: "auth-pk" }); // addVirtualAuthenticator

    const id = await mgr.setupForPasskeys();
    expect(id).toBe("auth-pk");
    expect(cdp.send).toHaveBeenNthCalledWith(1, "WebAuthn.enable", { enableUI: false });
    expect(cdp.send).toHaveBeenNthCalledWith(
      2,
      "WebAuthn.addVirtualAuthenticator",
      expect.objectContaining({
        options: expect.objectContaining({ protocol: "ctap2", transport: "internal" }),
      }),
    );
  });

  it("injectCredentials() calls setupForPasskeys + addCredential for each", async () => {
    cdp.send
      .mockResolvedValueOnce(undefined) // enable
      .mockResolvedValueOnce({ authenticatorId: "auth-pk" }) // addVirtualAuthenticator
      .mockResolvedValue(undefined); // addCredential calls

    const creds = [makeCred(), makeCred({ credentialId: "cred-2" })];
    await mgr.injectCredentials(creds);

    // enable + addVirtualAuthenticator + 2x addCredential
    expect(cdp.send).toHaveBeenCalledTimes(4);
    expect(cdp.send).toHaveBeenNthCalledWith(3, "WebAuthn.addCredential", {
      authenticatorId: "auth-pk",
      credential: creds[0],
    });
    expect(cdp.send).toHaveBeenNthCalledWith(4, "WebAuthn.addCredential", {
      authenticatorId: "auth-pk",
      credential: creds[1],
    });
  });

  it("extractCredentials() calls getCredentials from default authenticator", async () => {
    cdp.send
      .mockResolvedValueOnce(undefined) // enable
      .mockResolvedValueOnce({ authenticatorId: "auth-pk" }) // addVirtualAuthenticator
      .mockResolvedValueOnce({ credentials: [makeCred()] }); // getCredentials

    await mgr.setupForPasskeys();
    const creds = await mgr.extractCredentials();
    expect(creds).toHaveLength(1);
    expect(cdp.send).toHaveBeenLastCalledWith("WebAuthn.getCredentials", {
      authenticatorId: "auth-pk",
    });
  });

  it("extractCredentials() throws when no default authenticator exists", async () => {
    await expect(mgr.extractCredentials()).rejects.toThrow("No default authenticator");
  });

  it("teardown() calls removeAuthenticator + disable", async () => {
    cdp.send
      .mockResolvedValueOnce(undefined) // enable
      .mockResolvedValueOnce({ authenticatorId: "auth-pk" }) // addVirtualAuthenticator
      .mockResolvedValue(undefined); // remove + disable

    await mgr.setupForPasskeys();
    cdp.send.mockClear();

    await mgr.teardown();
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.removeVirtualAuthenticator", {
      authenticatorId: "auth-pk",
    });
    expect(cdp.send).toHaveBeenCalledWith("WebAuthn.disable");
  });

  // ── error handling ──────────────────────────────────────────────────────

  it("wraps CDP errors with descriptive messages", async () => {
    cdp.send.mockRejectedValue(new Error("CDP timeout"));
    await expect(mgr.enable()).rejects.toThrow("WebAuthn.enable failed: CDP timeout");
    await expect(mgr.disable()).rejects.toThrow("WebAuthn.disable failed: CDP timeout");
    await expect(mgr.addAuthenticator()).rejects.toThrow(
      "WebAuthn.addVirtualAuthenticator failed: CDP timeout",
    );
    await expect(mgr.addCredential("a", makeCred())).rejects.toThrow(
      "WebAuthn.addCredential failed: CDP timeout",
    );
    await expect(mgr.getCredentials("a")).rejects.toThrow(
      "WebAuthn.getCredentials failed: CDP timeout",
    );
    await expect(mgr.removeCredential("a", "c")).rejects.toThrow(
      "WebAuthn.removeCredential failed: CDP timeout",
    );
    await expect(mgr.clearCredentials("a")).rejects.toThrow(
      "WebAuthn.clearCredentials failed: CDP timeout",
    );
    await expect(mgr.setUserVerified("a", true)).rejects.toThrow(
      "WebAuthn.setUserVerified failed: CDP timeout",
    );
  });
});
