/**
 * E2E Passkey Flow (M1-I9)
 * Tests WebAuthn virtual authenticator + PasskeyStore round-trip
 * using a real Chromium browser via Playwright.
 */
import { describe, it, expect, afterAll, beforeAll } from 'vitest';
import { chromium, type Browser } from 'playwright';
import { WebAuthnManager, type WebAuthnCredential } from '../../src/auth/webauthn-manager.js';
import { PasskeyStore } from '../../src/auth/passkey-store.js';
import { join } from 'node:path';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { generateKeyPairSync } from 'node:crypto';

let browser: Browser;
let tmpDir: string;

/** Generate a valid PKCS#8 EC P-256 private key, base64-encoded (required by CDP). */
function generateValidPrivateKey(): string {
  const { privateKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' });
  const der = privateKey.export({ type: 'pkcs8', format: 'der' });
  return Buffer.from(der).toString('base64');
}

beforeAll(async () => {
  browser = await chromium.launch({ headless: true });
});

afterAll(async () => {
  await browser?.close().catch(() => {});
  if (tmpDir) await rm(tmpDir, { recursive: true, force: true }).catch(() => {});
});

describe('Passkey E2E Flow', () => {
  it('can enable WebAuthn on a real CDP session and extract empty credentials', async () => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    const cdpSession = await ctx.newCDPSession(page);
    const manager = new WebAuthnManager(cdpSession);

    const authId = await manager.setupForPasskeys();
    expect(authId).toBeTruthy();
    expect(typeof authId).toBe('string');

    const creds = await manager.extractCredentials();
    expect(creds).toEqual([]);

    await manager.teardown();
    await page.close();
    await ctx.close();
  });

  it('can add and retrieve credentials', async () => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    const cdpSession = await ctx.newCDPSession(page);
    const manager = new WebAuthnManager(cdpSession);

    const authId = await manager.setupForPasskeys();

    const testCredential: WebAuthnCredential = {
      credentialId: btoa('test-credential-id-123'),
      isResidentCredential: true,
      rpId: 'example.com',
      privateKey: generateValidPrivateKey(),
      userHandle: btoa('user-handle-123'),
      signCount: 0,
    };

    await manager.addCredential(authId, testCredential);

    const creds = await manager.getCredentials(authId);
    expect(creds.length).toBe(1);
    expect(creds[0].rpId).toBe('example.com');
    expect(creds[0].isResidentCredential).toBe(true);

    await manager.teardown();
    await page.close();
    await ctx.close();
  });

  // Network-dependent test: webauthn.io may be flaky — skipped by default
  it.skip('can register passkey on webauthn.io (network-dependent)', async () => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    const cdpSession = await ctx.newCDPSession(page);
    const manager = new WebAuthnManager(cdpSession);

    await manager.setupForPasskeys();

    await page.goto('https://webauthn.io', { waitUntil: 'networkidle' });

    const usernameInput = page.locator('input[name="username"]');
    await usernameInput.fill(`e2e-test-${Date.now()}`);

    const registerBtn = page.locator('button:has-text("Register")');
    await registerBtn.click();

    await page.waitForTimeout(3000);

    const creds = await manager.extractCredentials();
    expect(creds.length).toBeGreaterThan(0);
    expect(creds[0].credentialId).toBeTruthy();
    expect(creds[0].privateKey).toBeTruthy();

    await manager.teardown();
    await page.close();
    await ctx.close();
  });

  it('PasskeyStore round-trip with real encryption', async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'onecrawl-passkey-test-'));
    const storePath = join(tmpDir, 'passkey.json');
    const keyPath = join(tmpDir, 'key');

    const store = new PasskeyStore({ storagePath: storePath, keyPath });

    // Initially empty
    const initial = await store.load();
    expect(initial).toBeNull();
    expect(await store.exists()).toBe(false);

    // --- Browser session 1: add credential and extract ---
    const ctx1 = await browser.newContext();
    const page1 = await ctx1.newPage();
    const cdp1 = await ctx1.newCDPSession(page1);
    const manager1 = new WebAuthnManager(cdp1);
    await manager1.setupForPasskeys();

    const cred: WebAuthnCredential = {
      credentialId: btoa('round-trip-cred-001'),
      isResidentCredential: true,
      rpId: 'test.example.com',
      privateKey: generateValidPrivateKey(),
      userHandle: btoa('user-42'),
      signCount: 5,
    };

    await manager1.injectCredentials([cred]);
    const extracted = await manager1.extractCredentials();
    expect(extracted.length).toBe(1);

    // Save to PasskeyStore
    await store.addCredential(extracted[0], 'test.example.com');
    expect(await store.exists()).toBe(true);

    await manager1.teardown();
    await page1.close();
    await ctx1.close();

    // --- Load back from store ---
    const loaded = await store.getCredentials('test.example.com');
    expect(loaded.length).toBe(1);
    expect(loaded[0].rpId).toBe('test.example.com');
    expect(loaded[0].isResidentCredential).toBe(true);

    // --- Browser session 2: inject into a new context ---
    const ctx2 = await browser.newContext();
    const page2 = await ctx2.newPage();
    const cdp2 = await ctx2.newCDPSession(page2);
    const manager2 = new WebAuthnManager(cdp2);
    await manager2.setupForPasskeys();

    await manager2.injectCredentials(loaded);

    const verified = await manager2.extractCredentials();
    expect(verified.length).toBe(1);
    expect(verified[0].rpId).toBe('test.example.com');

    await manager2.teardown();
    await page2.close();
    await ctx2.close();

    // Cleanup store
    await store.clear();
    expect(await store.exists()).toBe(false);
  });
});
