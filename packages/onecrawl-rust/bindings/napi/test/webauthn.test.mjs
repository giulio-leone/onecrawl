import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('WebAuthn Virtual Authenticator', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('enableVirtualAuthenticator does not throw', async () => {
    await b.goto('data:text/html,<h1>WebAuthn</h1>');
    const config = JSON.stringify({
      id: 'test-auth',
      protocol: 'ctap2',
      transport: 'internal',
      has_resident_key: true,
      has_user_verification: true,
      is_user_verified: true,
    });
    await assert.doesNotReject(() => b.enableVirtualAuthenticator(config));
  });

  it('getVirtualCredentials returns empty array initially', async () => {
    const json = await b.getVirtualCredentials();
    const creds = JSON.parse(json);
    assert.ok(Array.isArray(creds));
    assert.equal(creds.length, 0);
  });

  it('addVirtualCredential adds a credential', async () => {
    const cred = JSON.stringify({
      credential_id: 'dGVzdC1jcmVk',
      rp_id: 'example.com',
      user_handle: 'dXNlcjE',
      sign_count: 0,
    });
    await assert.doesNotReject(() => b.addVirtualCredential(cred));
  });

  it('getVirtualCredentials returns the added credential', async () => {
    const json = await b.getVirtualCredentials();
    const creds = JSON.parse(json);
    assert.equal(creds.length, 1);
    assert.equal(creds[0].credential_id, 'dGVzdC1jcmVk');
    assert.equal(creds[0].rp_id, 'example.com');
  });

  it('addVirtualCredential supports multiple credentials', async () => {
    const cred2 = JSON.stringify({
      credential_id: 'c2Vjb25k',
      rp_id: 'other.com',
      user_handle: 'dXNlcjI',
      sign_count: 5,
    });
    await b.addVirtualCredential(cred2);
    const json = await b.getVirtualCredentials();
    const creds = JSON.parse(json);
    assert.equal(creds.length, 2);
  });

  it('getWebauthnLog returns valid JSON array', async () => {
    const json = await b.getWebauthnLog();
    const log = JSON.parse(json);
    assert.ok(Array.isArray(log));
  });

  it('removeVirtualCredential removes an existing credential', async () => {
    const removed = await b.removeVirtualCredential('dGVzdC1jcmVk');
    assert.equal(removed, true);
    const json = await b.getVirtualCredentials();
    const creds = JSON.parse(json);
    assert.equal(creds.length, 1);
    assert.equal(creds[0].credential_id, 'c2Vjb25k');
  });

  it('removeVirtualCredential returns false for non-existent credential', async () => {
    const removed = await b.removeVirtualCredential('nonexistent');
    assert.equal(removed, false);
  });

  it('disableVirtualAuthenticator does not throw', async () => {
    await assert.doesNotReject(() => b.disableVirtualAuthenticator());
  });

  it('getVirtualCredentials returns empty after disable', async () => {
    const json = await b.getVirtualCredentials();
    const creds = JSON.parse(json);
    assert.equal(creds.length, 0);
  });

  it('enableVirtualAuthenticator with u2f protocol works', async () => {
    const config = JSON.stringify({
      id: 'u2f-auth',
      protocol: 'u2f',
      transport: 'usb',
      has_resident_key: false,
      has_user_verification: false,
      is_user_verified: false,
    });
    await assert.doesNotReject(() => b.enableVirtualAuthenticator(config));
    await b.disableVirtualAuthenticator();
  });
});
