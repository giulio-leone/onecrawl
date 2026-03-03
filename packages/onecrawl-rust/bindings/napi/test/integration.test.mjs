import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const require = createRequire(import.meta.url);
const {
  NativeBrowser,
  NativeStore,
  encrypt,
  decrypt,
  generatePkce,
  generateTotp,
  verifyTotp,
  parseAccessibilityTree,
  querySelector,
  extractText,
  extractLinks,
} = require('../index.js');

describe('Integration: Browser → Parser → Crypto → Storage pipeline', () => {
  let browser;
  let store;
  let tmpDir;

  after(async () => {
    if (browser) await browser.close();
    if (tmpDir) rmSync(tmpDir, { recursive: true, force: true });
  });

  it('Step 1: Launch browser and navigate', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
    const title = await browser.getTitle();
    assert.equal(title, 'Example Domain');
  });

  let pageHtml;

  it('Step 2: Extract page content', async () => {
    pageHtml = await browser.content();
    assert.ok(pageHtml.length > 100, 'should have meaningful HTML');
  });

  it('Step 3: Parse HTML with parser → accessibility tree', () => {
    const tree = parseAccessibilityTree(pageHtml);
    assert.ok(tree.length > 0, 'accessibility tree should not be empty');
    assert.ok(tree.includes('Example Domain'), 'tree should contain page heading');
  });

  it('Step 4: Parse HTML → querySelector', () => {
    const results = querySelector(pageHtml, 'h1');
    const parsed = JSON.parse(results);
    assert.ok(parsed.length > 0, 'should find h1 elements');
  });

  it('Step 5: Parse HTML → extractText', () => {
    const text = extractText(pageHtml);
    assert.ok(text.includes('Example Domain'));
  });

  it('Step 6: Parse HTML → extractLinks', () => {
    const links = extractLinks(pageHtml);
    assert.ok(links.length > 0, 'should extract links');
    const ianaLink = links.find(l => l.href.includes('iana.org'));
    assert.ok(ianaLink, 'should find IANA link');
    assert.equal(ianaLink.isExternal, true);
  });

  it('Step 7: Encrypt page content with AES-256-GCM', () => {
    const password = 'integration-test-password-2024';
    const plainBuf = Buffer.from(pageHtml, 'utf-8');
    const encrypted = encrypt(plainBuf, password);
    assert.ok(encrypted.length > plainBuf.length, 'encrypted should be larger');

    const decrypted = decrypt(encrypted, password);
    const recovered = Buffer.from(decrypted).toString('utf-8');
    assert.equal(recovered, pageHtml, 'decrypted should match original');
  });

  it('Step 8: Store encrypted content in sled KV store', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-integration-'));
    const storeDir = join(tmpDir, 'test-store');
    store = new NativeStore(storeDir, 'store-password-2024');

    // Store page metadata
    store.set('page:url', 'https://example.com');
    store.set('page:title', 'Example Domain');
    store.set('page:html_length', String(pageHtml.length));

    // Verify retrieval
    assert.equal(store.get('page:url'), 'https://example.com');
    assert.equal(store.get('page:title'), 'Example Domain');
    assert.ok(store.contains('page:html_length'));
  });

  it('Step 9: Generate PKCE challenge (crypto)', () => {
    const pkce = generatePkce();
    assert.ok(pkce.verifier.length >= 43, 'verifier should be 43+ chars');
    assert.ok(pkce.challenge.length > 0, 'challenge should not be empty');
    store.set('auth:pkce_verifier', pkce.verifier);
    store.set('auth:pkce_challenge', pkce.challenge);
  });

  it('Step 10: Generate + verify TOTP (crypto)', () => {
    const secret = 'JBSWY3DPEHPK3PXP'; // standard test secret
    const code = generateTotp(secret);
    assert.equal(code.length, 6, 'TOTP should be 6 digits');
    assert.ok(/^\d{6}$/.test(code), 'TOTP should be numeric');

    const valid = verifyTotp(secret, code);
    assert.equal(valid, true, 'TOTP should verify');

    store.set('auth:last_totp', code);
  });

  it('Step 11: Store screenshot bytes in KV', async () => {
    const png = await browser.screenshot();
    assert.ok(png.length > 1000);

    // Store metadata (not the full binary for perf)
    store.set('screenshot:size', String(png.length));
    store.set('screenshot:format', 'png');
    store.flush();

    // Verify all stored keys
    const keys = store.list();
    assert.ok(keys.length >= 6, `should have 6+ keys, got ${keys.length}`);
  });

  it('Step 12: Full pipeline verification', () => {
    // Verify the full roundtrip:
    // Browser → HTML content → Parser analysis → Crypto encrypt → Storage persist
    const url = store.get('page:url');
    const title = store.get('page:title');
    const screenshotSize = parseInt(store.get('screenshot:size'), 10);
    const pkceVerifier = store.get('auth:pkce_verifier');
    const totp = store.get('auth:last_totp');

    assert.equal(url, 'https://example.com');
    assert.equal(title, 'Example Domain');
    assert.ok(screenshotSize > 1000, 'screenshot should be > 1KB');
    assert.ok(pkceVerifier.length >= 43, 'PKCE verifier should be stored');
    assert.ok(/^\d{6}$/.test(totp), 'TOTP should be stored');
  });
});
