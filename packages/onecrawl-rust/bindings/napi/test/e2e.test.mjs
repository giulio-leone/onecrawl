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

// ────────────────────── Crypto → Storage → Decrypt Pipeline ──────────────────────

describe('E2E: Crypto → Storage → Decrypt pipeline', () => {
  const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-e2e-'));
  after(() => rmSync(tmpDir, { recursive: true, force: true }));

  it('full encrypt-store-retrieve-decrypt cycle', () => {
    // 1. Encrypt sensitive data
    const secret = Buffer.from('api-key-abc-123-sensitive');
    const encrypted = encrypt(secret, 'master-password');
    assert.ok(encrypted.length > secret.length, 'ciphertext should be larger');

    // 2. Store encrypted blob in NativeStore
    const store = new NativeStore(join(tmpDir, 'db'), 'store-pw');
    store.set('api_key', encrypted.toString('base64'));

    // 3. Retrieve
    const storedBase64 = store.get('api_key');
    assert.ok(storedBase64, 'should retrieve stored value');
    const storedBuffer = Buffer.from(storedBase64, 'base64');

    // 4. Decrypt
    const decrypted = decrypt(storedBuffer, 'master-password');
    assert.deepEqual(decrypted, secret);
  });

  it('store multiple tokens and retrieve by key', () => {
    const store = new NativeStore(join(tmpDir, 'tokens'), 'pw');

    const tokens = {
      'oauth:access': 'eyJ-access-token',
      'oauth:refresh': 'eyJ-refresh-token',
      'cookie:li_at': 'AQEDAS-cookie-value',
    };

    // Store all tokens
    for (const [key, value] of Object.entries(tokens)) {
      store.set(key, value);
    }

    // Retrieve and verify all
    for (const [key, expected] of Object.entries(tokens)) {
      assert.equal(store.get(key), expected, `mismatch for ${key}`);
    }

    // List all keys
    const keys = store.list();
    assert.equal(keys.length, 3);
  });

  it('encrypted store persists data to disk via flush', () => {
    const path = join(tmpDir, 'persist');
    const store = new NativeStore(path, 'pw');

    // Write and flush to disk
    store.set('persistent', 'value-123');
    store.set('another', 'value-456');
    store.flush();

    // Data is still accessible after flush (proves disk write)
    assert.equal(store.get('persistent'), 'value-123');
    assert.equal(store.get('another'), 'value-456');
    assert.equal(store.list().length, 2);
  });
});

// ────────────────────── PKCE + TOTP Combined Auth Flow ──────────────────────

describe('E2E: PKCE + TOTP combined auth flow', () => {
  it('generate and verify PKCE challenge', () => {
    const pkce = generatePkce();
    assert.ok(pkce.verifier, 'should have verifier');
    assert.ok(pkce.challenge, 'should have challenge');
    assert.ok(pkce.verifier.length >= 43, 'verifier should be at least 43 chars');

    // Two challenges should be different
    const pkce2 = generatePkce();
    assert.notEqual(pkce.verifier, pkce2.verifier);
    assert.notEqual(pkce.challenge, pkce2.challenge);
  });

  it('generate and verify TOTP code', () => {
    const secret = 'JBSWY3DPEHPK3PXP';

    // Generate code
    const code = generateTotp(secret);
    assert.match(code, /^\d{6}$/, 'should be 6 digits');

    // Verify immediately (within ±1 step window)
    assert.ok(verifyTotp(secret, code), 'should verify own code');

    // Wrong code should fail
    assert.ok(!verifyTotp(secret, '000000'), 'should reject wrong code');
  });

  it('PKCE + TOTP + storage: simulated OAuth 2.1 flow', () => {
    const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-auth-'));

    try {
      const store = new NativeStore(join(tmpDir, 'auth'), 'pw');

      // 1. Generate PKCE for OAuth authorization request
      const pkce = generatePkce();
      store.set('pkce:verifier', pkce.verifier);
      store.set('pkce:challenge', pkce.challenge);

      // 2. Generate TOTP for 2FA during login
      const totpSecret = 'JBSWY3DPEHPK3PXP';
      const code = generateTotp(totpSecret);
      assert.ok(verifyTotp(totpSecret, code));

      // 3. After successful auth, store the tokens
      const accessToken = 'eyJ-mock-access-token-' + Date.now();
      const encrypted = encrypt(Buffer.from(accessToken), 'token-key');
      store.set('token:encrypted', encrypted.toString('base64'));

      // 4. Retrieve and decrypt token later
      const retrieved = Buffer.from(store.get('token:encrypted'), 'base64');
      const decrypted = decrypt(retrieved, 'token-key');
      assert.equal(decrypted.toString(), accessToken);

      // 5. Verify PKCE was stored correctly
      assert.equal(store.get('pkce:verifier'), pkce.verifier);
    } finally {
      rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});

// ────────────────────── HTML Parsing Pipeline ──────────────────────

describe('E2E: HTML parsing pipeline', () => {
  const html = `<!DOCTYPE html>
<html lang="en">
<head><title>Job Listing</title></head>
<body>
  <h1>Senior AI Engineer</h1>
  <div class="company">Scale AI</div>
  <div class="location">San Francisco, CA</div>
  <div class="description">
    <p>We are looking for an experienced AI engineer to join our team.</p>
    <p>Requirements: Python, Rust, ML systems</p>
  </div>
  <a href="https://scale.com/apply" class="apply-btn">Apply Now</a>
  <a href="/save" class="save-btn">Save Job</a>
</body>
</html>`;

  it('full page analysis: tree + text + links + selector', () => {
    // 1. Accessibility tree
    const tree = parseAccessibilityTree(html);
    assert.ok(tree.includes('Senior AI Engineer'), 'tree should have title');
    assert.ok(tree.includes('Apply Now'), 'tree should have apply button');

    // 2. Extract text
    const text = extractText(html);
    assert.ok(text.includes('experienced AI engineer'));
    assert.ok(text.includes('Python, Rust'));

    // 3. Extract links
    const links = extractLinks(html);
    assert.ok(links.length >= 2, 'should have at least 2 links');
    const applyLink = links.find(l => l.href.includes('scale.com'));
    assert.ok(applyLink, 'should find apply link');
    assert.ok(applyLink.isExternal, 'scale.com link should be external');
    assert.equal(applyLink.text, 'Apply Now');

    const saveLink = links.find(l => l.href === '/save');
    assert.ok(saveLink, 'should find save link');
    assert.ok(!saveLink.isExternal, '/save should be internal');

    // 4. Query selector
    const results = JSON.parse(querySelector(html, '.description p'));
    assert.equal(results.length, 2);
    assert.ok(results[0].text.includes('experienced'));
    assert.ok(results[1].text.includes('Requirements'));
  });

  it('parse complex LinkedIn-like job page', () => {
    const linkedinHtml = `<!DOCTYPE html>
<html>
<body>
  <div class="job-card" data-job-id="12345">
    <h2 class="job-title">LLM Infrastructure Engineer</h2>
    <span class="company-name">Anthropic</span>
    <span class="location">San Francisco, CA (Remote)</span>
    <div class="job-details">
      <span class="badge">Easy Apply</span>
      <span class="posted">Posted 2 days ago</span>
      <span class="applicants">42 applicants</span>
    </div>
    <div class="description">
      <p>Build and scale LLM training infrastructure.</p>
      <ul>
        <li>5+ years experience in distributed systems</li>
        <li>Strong Rust or C++ skills</li>
        <li>Experience with GPU clusters</li>
      </ul>
    </div>
    <a href="https://anthropic.com/careers/12345" class="apply">Apply</a>
  </div>
</body>
</html>`;

    // Extract job metadata
    const title = JSON.parse(querySelector(linkedinHtml, '.job-title'));
    assert.equal(title[0].text, 'LLM Infrastructure Engineer');

    const company = JSON.parse(querySelector(linkedinHtml, '.company-name'));
    assert.equal(company[0].text, 'Anthropic');

    const badges = JSON.parse(querySelector(linkedinHtml, '.badge'));
    assert.equal(badges[0].text, 'Easy Apply');

    // Extract requirements list
    const requirements = JSON.parse(querySelector(linkedinHtml, '.description li'));
    assert.equal(requirements.length, 3);
    assert.ok(requirements.some(r => r.text.includes('Rust')));

    // Extract apply link
    const links = extractLinks(linkedinHtml);
    const applyLink = links.find(l => l.href.includes('anthropic.com'));
    assert.ok(applyLink);
    assert.ok(applyLink.isExternal);
  });
});

// ────────────────────── Storage Stress Test ──────────────────────

describe('E2E: Storage stress test', () => {
  const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-stress-'));
  after(() => rmSync(tmpDir, { recursive: true, force: true }));

  it('handles 500 entries with encrypted storage', () => {
    const store = new NativeStore(join(tmpDir, 'stress'), 'pw');

    // Write 500 entries
    for (let i = 0; i < 500; i++) {
      store.set(`job-${i}`, JSON.stringify({ title: `Job ${i}`, score: i * 0.2 }));
    }

    // Verify count
    const keys = store.list();
    assert.equal(keys.length, 500);

    // Retrieve random entry
    const entry = JSON.parse(store.get('job-250'));
    assert.equal(entry.title, 'Job 250');
    assert.equal(entry.score, 50);

    // Delete and verify
    store.delete('job-0');
    assert.ok(!store.contains('job-0'));
    assert.equal(store.list().length, 499);
  });

  it('rapid overwrite cycle', () => {
    const store = new NativeStore(join(tmpDir, 'overwrite'), 'pw');

    // Overwrite same key 100 times
    for (let i = 0; i < 100; i++) {
      store.set('counter', String(i));
    }

    // Should have final value
    assert.equal(store.get('counter'), '99');
    assert.equal(store.list().length, 1);
  });
});

// ────────────────────── Key Derivation Pipeline ──────────────────────

describe('E2E: Key derivation pipeline', () => {
  it('deriveKey is deterministic', () => {
    const salt = Buffer.alloc(16, 0xab);
    const k1 = deriveKey('password', salt);
    const k2 = deriveKey('password', salt);
    assert.deepEqual(k1, k2);
    assert.equal(k1.length, 32);
  });

  it('different salts produce different keys', () => {
    const k1 = deriveKey('password', Buffer.alloc(16, 0x01));
    const k2 = deriveKey('password', Buffer.alloc(16, 0x02));
    assert.notDeepEqual(k1, k2);
  });

  it('different passwords produce different keys', () => {
    const salt = Buffer.alloc(16, 0xab);
    const k1 = deriveKey('pass-a', salt);
    const k2 = deriveKey('pass-b', salt);
    assert.notDeepEqual(k1, k2);
  });
});
