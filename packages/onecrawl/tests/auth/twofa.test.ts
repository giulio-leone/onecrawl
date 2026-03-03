/**
 * Two-Factor Authentication module unit tests (M14)
 */

import { describe, it, expect, afterEach } from "vitest";
import { mkdtemp, rm, access } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

import {
  base32Decode,
  generateTOTP,
  verifyTOTP,
} from "../../src/auth/totp-adapter.js";

import { TotpSecretStore } from "../../src/auth/totp-secret-store.js";
import { detectChallenge } from "../../src/auth/twofa-detector.js";
import { Sms2faAdapter } from "../../src/auth/sms-2fa-adapter.js";

// =============================================================================
// Base32 Decoding
// =============================================================================

describe("base32Decode", () => {
  it("decodes known values correctly", () => {
    // "JBSWY3DPEE" → "Hello!" (48 65 6c 6c 6f 21)
    const buf = base32Decode("JBSWY3DPEE");
    expect(buf.toString("ascii")).toBe("Hello!");
  });

  it("decodes RFC 6238 test secret", () => {
    // "12345678901234567890" (20 bytes) → base32 "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"
    const buf = base32Decode("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
    expect(buf.toString("ascii")).toBe("12345678901234567890");
    expect(buf.length).toBe(20);
  });

  it("handles lowercase and padding", () => {
    const buf = base32Decode("gezdgnbvgy3tqojq====");
    expect(buf.toString("ascii")).toBe("1234567890");
  });

  it("throws on invalid characters", () => {
    expect(() => base32Decode("INVALID!@#")).toThrow("Invalid base32 character");
  });
});

// =============================================================================
// TOTP Generation — RFC 6238 Appendix B (SHA1)
// =============================================================================

describe("generateTOTP", () => {
  // RFC 6238 test secret: ASCII "12345678901234567890" (20 bytes)
  const SECRET_B32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

  it("produces correct code at time=59 (counter=1)", () => {
    const code = generateTOTP(SECRET_B32, { time: 59 });
    expect(code).toBe("287082");
  });

  it("produces correct code at time=1111111109", () => {
    const code = generateTOTP(SECRET_B32, { time: 1111111109 });
    expect(code).toBe("081804");
  });

  it("produces correct code at time=1111111111", () => {
    const code = generateTOTP(SECRET_B32, { time: 1111111111 });
    expect(code).toBe("050471");
  });

  it("produces correct code at time=1234567890", () => {
    const code = generateTOTP(SECRET_B32, { time: 1234567890 });
    expect(code).toBe("005924");
  });

  it("produces correct code at time=2000000000", () => {
    const code = generateTOTP(SECRET_B32, { time: 2000000000 });
    expect(code).toBe("279037");
  });

  it("produces correct code at time=20000000000", () => {
    const code = generateTOTP(SECRET_B32, { time: 20000000000 });
    expect(code).toBe("353130");
  });

  it("returns a 6-digit zero-padded string", () => {
    const code = generateTOTP(SECRET_B32, { time: 0 });
    expect(code).toMatch(/^\d{6}$/);
  });
});

// =============================================================================
// TOTP Verification
// =============================================================================

describe("verifyTOTP", () => {
  const SECRET_B32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

  it("accepts a valid code at the exact time step", () => {
    const code = generateTOTP(SECRET_B32, { time: 59 });
    // verifyTOTP with testTime at the same step
    expect(verifyTOTP(SECRET_B32, code, 1, 59)).toBe(true);
  });

  it("accepts a code within ±1 window tolerance", () => {
    // Generate code for counter=1 (time step 30-59)
    const code = generateTOTP(SECRET_B32, { time: 59 });
    // Verify from the next time step (counter=2, time=60-89)
    expect(verifyTOTP(SECRET_B32, code, 1, 75)).toBe(true);
  });

  it("rejects an expired code outside the window", () => {
    // Code at counter=1
    const code = generateTOTP(SECRET_B32, { time: 59 });
    // Verify at counter=10 — way outside ±1 window
    expect(verifyTOTP(SECRET_B32, code, 1, 300)).toBe(false);
  });

  it("rejects a completely wrong code", () => {
    expect(verifyTOTP(SECRET_B32, "000000", 1, 59)).toBe(false);
  });

  it("respects custom window size", () => {
    const code = generateTOTP(SECRET_B32, { time: 59 }); // counter=1
    // At time 150 → counter=5, distance=4 from counter=1
    expect(verifyTOTP(SECRET_B32, code, 1, 150)).toBe(false);
    expect(verifyTOTP(SECRET_B32, code, 5, 150)).toBe(true);
  });
});

// =============================================================================
// TOTP Secret Store
// =============================================================================

describe("TotpSecretStore", () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) {
      await rm(tmpDir, { recursive: true, force: true });
    }
  });

  async function createStore() {
    tmpDir = await mkdtemp(join(tmpdir(), "totp-store-test-"));
    const storagePath = join(tmpDir, "totp-secret.json");
    const keyPath = join(tmpDir, "key");
    return new TotpSecretStore({ storagePath, keyPath });
  }

  it("saveSecret + getSecret round-trip", async () => {
    const store = await createStore();
    await store.saveSecret("JBSWY3DPEHPK3PXP");
    const loaded = await store.getSecret();
    expect(loaded).toBe("JBSWY3DPEHPK3PXP");
  });

  it("getSecret returns null when no secret exists", async () => {
    const store = await createStore();
    expect(await store.getSecret()).toBeNull();
  });

  it("hasSecret reflects stored state", async () => {
    const store = await createStore();
    expect(await store.hasSecret()).toBe(false);
    await store.saveSecret("SECRET");
    expect(await store.hasSecret()).toBe(true);
  });

  it("clearSecret removes the file", async () => {
    const store = await createStore();
    await store.saveSecret("SECRET");
    await store.clearSecret();
    expect(await store.hasSecret()).toBe(false);
    expect(await store.getSecret()).toBeNull();
  });

  it("clearSecret is safe when no file exists", async () => {
    const store = await createStore();
    await expect(store.clearSecret()).resolves.not.toThrow();
  });

  it("encrypted file is not plaintext", async () => {
    const store = await createStore();
    await store.saveSecret("MYSECRET");
    const storagePath = join(tmpDir, "totp-secret.json");
    const { readFile } = await import("fs/promises");
    const raw = await readFile(storagePath, "utf-8");
    expect(raw).not.toContain("MYSECRET");
    const parsed = JSON.parse(raw);
    expect(parsed).toHaveProperty("iv");
    expect(parsed).toHaveProperty("salt");
    expect(parsed).toHaveProperty("data");
    expect(parsed).toHaveProperty("tag");
  });
});

// =============================================================================
// 2FA Challenge Detector
// =============================================================================

describe("detectChallenge", () => {
  it("detects TOTP authenticator challenge", () => {
    const html = `
      <div class="challenge">
        <h1>Two-step verification</h1>
        <p>Enter the code from your authenticator app</p>
        <input id="code" />
      </div>
    `;
    const result = detectChallenge(html);
    expect(result).not.toBeNull();
    expect(result!.method).toBe("totp");
  });

  it("detects SMS challenge with phone hint", () => {
    const html = `
      <div class="challenge">
        <h1>Verify your identity</h1>
        <p>Enter the code we sent to ***-***-4589</p>
        <input id="code" />
      </div>
    `;
    const result = detectChallenge(html);
    expect(result).not.toBeNull();
    expect(result!.method).toBe("sms");
    expect(result!.hint).toContain("4589");
  });

  it("detects email challenge with email hint", () => {
    const html = `
      <div class="challenge">
        <h1>Verify your identity</h1>
        <p>Enter the code we sent to g***@gmail.com</p>
        <input id="code" />
      </div>
    `;
    const result = detectChallenge(html);
    expect(result).not.toBeNull();
    expect(result!.method).toBe("email");
    expect(result!.hint).toContain("@gmail.com");
  });

  it("returns null for non-2FA pages", () => {
    const html = `
      <div class="feed">
        <h1>Welcome to LinkedIn</h1>
        <p>Your feed is ready</p>
      </div>
    `;
    expect(detectChallenge(html)).toBeNull();
  });

  it("returns null for empty content", () => {
    expect(detectChallenge("")).toBeNull();
  });

  it("detects alternative SMS wording", () => {
    const html = `<p>We texted a code to +1 (555) ***-1234</p>`;
    const result = detectChallenge(html);
    expect(result).not.toBeNull();
    expect(result!.method).toBe("sms");
  });

  it("detects alternative email wording", () => {
    const html = `<p>Check your email for a verification code</p>`;
    const result = detectChallenge(html);
    expect(result).not.toBeNull();
    expect(result!.method).toBe("email");
  });
});

// =============================================================================
// SMS 2FA Adapter
// =============================================================================

describe("Sms2faAdapter", () => {
  it("invokes onCodeRequired callback in getCode()", async () => {
    const adapter = new Sms2faAdapter({
      onCodeRequired: async () => "123456",
      getPageContent: async () => "",
      submitCodeToPage: async () => true,
    });

    const code = await adapter.getCode("sms");
    expect(code).toBe("123456");
  });

  it("detect() returns SMS challenge when page matches", async () => {
    const adapter = new Sms2faAdapter({
      onCodeRequired: async () => "",
      getPageContent: async () =>
        `<p>Enter the code we sent to ***-***-9876</p>`,
      submitCodeToPage: async () => true,
    });

    const challenge = await adapter.detect();
    expect(challenge).not.toBeNull();
    expect(challenge!.method).toBe("sms");
  });

  it("detect() returns null for non-SMS challenges", async () => {
    const adapter = new Sms2faAdapter({
      onCodeRequired: async () => "",
      getPageContent: async () =>
        `<p>Enter the code from your authenticator app</p>`,
      submitCodeToPage: async () => true,
    });

    expect(await adapter.detect()).toBeNull();
  });

  it("submitCode() delegates to submitCodeToPage", async () => {
    let submittedCode = "";
    const adapter = new Sms2faAdapter({
      onCodeRequired: async () => "",
      getPageContent: async () => "",
      submitCodeToPage: async (code) => {
        submittedCode = code;
        return true;
      },
    });

    const result = await adapter.submitCode("654321");
    expect(result).toBe(true);
    expect(submittedCode).toBe("654321");
  });
});
