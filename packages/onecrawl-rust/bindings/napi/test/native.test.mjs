import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const require = createRequire(import.meta.url);
const {
  encrypt, decrypt, deriveKey, generatePkce, generateTotp, verifyTotp,
  parseAccessibilityTree, querySelector, extractText, extractLinks,
  NativeStore,
} = require('../index.js');

// --- Crypto ---

describe('Crypto', () => {
  it('encrypt + decrypt round-trip', () => {
    const plain = Buffer.from('Hello OneCrawl');
    const ct = encrypt(plain, 'test-password');
    assert.ok(ct.length > plain.length, 'ciphertext longer than plaintext');
    const decrypted = decrypt(ct, 'test-password');
    assert.deepEqual(decrypted, plain);
  });

  it('decrypt with wrong password throws', () => {
    const ct = encrypt(Buffer.from('secret'), 'correct');
    assert.throws(() => decrypt(ct, 'wrong'), /decrypt/i);
  });

  it('deriveKey returns 32 bytes', () => {
    const salt = Buffer.alloc(16, 0xab);
    const key = deriveKey('password', salt);
    assert.equal(key.length, 32);
  });

  it('deriveKey is deterministic', () => {
    const salt = Buffer.alloc(16, 0xcd);
    const k1 = deriveKey('pw', salt);
    const k2 = deriveKey('pw', salt);
    assert.deepEqual(k1, k2);
  });

  it('generatePkce returns verifier + challenge', () => {
    const pkce = generatePkce();
    assert.ok(pkce.verifier.length >= 43, 'verifier has sufficient length');
    assert.ok(pkce.challenge.length > 0, 'challenge is non-empty');
    assert.notEqual(pkce.verifier, pkce.challenge);
  });

  it('generateTotp returns 6-digit code', () => {
    const code = generateTotp('JBSWY3DPEHPK3PXP');
    assert.match(code, /^\d{6}$/);
  });

  it('verifyTotp validates own code', () => {
    const secret = 'JBSWY3DPEHPK3PXP';
    const code = generateTotp(secret);
    assert.ok(verifyTotp(secret, code));
  });
});

// --- Parser ---

const HTML = `<!DOCTYPE html>
<html lang="en"><head><title>Test</title></head>
<body>
  <h1>Hello</h1>
  <p>World <a href="https://example.com">link</a></p>
  <ul><li>One</li><li>Two</li></ul>
</body></html>`;

describe('Parser', () => {
  it('parseAccessibilityTree returns tree string', () => {
    const tree = parseAccessibilityTree(HTML);
    assert.ok(tree.includes('Hello'), 'contains heading text');
    assert.ok(tree.includes('link'), 'contains link text');
  });

  it('querySelector finds elements', () => {
    const result = JSON.parse(querySelector(HTML, 'li'));
    assert.equal(result.length, 2);
    assert.ok(result[0].text.includes('One'));
  });

  it('extractText returns visible text', () => {
    const text = extractText(HTML);
    assert.ok(text.includes('Hello'));
    assert.ok(text.includes('World'));
  });

  it('extractLinks finds anchors', () => {
    const links = extractLinks(HTML);
    assert.equal(links.length, 1);
    assert.equal(links[0].href, 'https://example.com');
    assert.equal(links[0].text, 'link');
    assert.equal(links[0].isExternal, true);
  });
});

// --- Storage ---

describe('NativeStore', () => {
  const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-napi-'));
  after(() => rmSync(tmpDir, { recursive: true, force: true }));

  it('set + get round-trip', () => {
    const store = new NativeStore(join(tmpDir, 'db1'), 'pw');
    store.set('key1', 'value1');
    assert.equal(store.get('key1'), 'value1');
  });

  it('get returns null for missing key', () => {
    const store = new NativeStore(join(tmpDir, 'db2'), 'pw');
    assert.equal(store.get('nonexistent'), null);
  });

  it('delete removes key', () => {
    const store = new NativeStore(join(tmpDir, 'db3'), 'pw');
    store.set('a', 'b');
    assert.ok(store.delete('a'));
    assert.equal(store.get('a'), null);
  });

  it('list returns all keys', () => {
    const store = new NativeStore(join(tmpDir, 'db4'), 'pw');
    store.set('x', '1');
    store.set('y', '2');
    const keys = store.list();
    assert.ok(keys.includes('x'));
    assert.ok(keys.includes('y'));
    assert.equal(keys.length, 2);
  });

  it('contains checks existence', () => {
    const store = new NativeStore(join(tmpDir, 'db5'), 'pw');
    store.set('exists', 'yes');
    assert.ok(store.contains('exists'));
    assert.ok(!store.contains('nope'));
  });

  it('flush does not throw', () => {
    const store = new NativeStore(join(tmpDir, 'db6'), 'pw');
    store.set('k', 'v');
    assert.doesNotThrow(() => store.flush());
  });
});
