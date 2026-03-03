import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const FORM_HTML = `data:text/html,
<html><body>
<form id="signup" action="/submit" method="POST">
  <label for="email">Email</label>
  <input id="email" name="email" type="email" placeholder="Enter email" required />
  <label for="name">Name</label>
  <input id="name" name="name" type="text" placeholder="Your name" />
  <input name="phone" type="tel" placeholder="Phone" />
  <select name="country"><option value="us">US</option><option value="it">IT</option></select>
  <textarea name="bio" placeholder="Bio"></textarea>
  <input name="agree" type="checkbox" />
  <input type="hidden" name="csrf" value="tok123" />
  <button type="submit">Sign Up</button>
</form>
</body></html>`;

const SITEMAP_XML = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>2024-01-01</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://example.com/about</loc>
    <changefreq>monthly</changefreq>
    <priority>0.5</priority>
  </url>
</urlset>`;

describe('Form Filler', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('detectForms finds the signup form', async () => {
    await b.goto(FORM_HTML);
    const forms = JSON.parse(await b.detectForms());
    assert.ok(Array.isArray(forms));
    assert.equal(forms.length, 1);
    assert.equal(forms[0].method, 'POST');
    assert.ok(forms[0].fields.length >= 5);
  });

  it('fillForm fills specified fields', async () => {
    await b.goto(FORM_HTML);
    const values = JSON.stringify({ email: 'test@x.com', name: 'Alice' });
    const result = JSON.parse(await b.fillForm('#signup', values));
    assert.ok(result.filled >= 2);
    assert.deepEqual(result.errors, []);
  });

  it('autoFillForm matches profile keys to fields', async () => {
    await b.goto(FORM_HTML);
    const profile = JSON.stringify({ email: 'a@b.com', name: 'Bob', phone: '123' });
    const result = JSON.parse(await b.autoFillForm('#signup', profile));
    assert.ok(result.filled >= 2);
  });

  it('submitForm submits the form', async () => {
    await b.goto(FORM_HTML);
    await b.submitForm('#signup');
    // Submission triggers navigation or no error
  });

  it('detectForms returns empty on page without forms', async () => {
    await b.goto('data:text/html,<html><body><p>No forms</p></body></html>');
    const forms = JSON.parse(await b.detectForms());
    assert.equal(forms.length, 0);
  });

  it('fillForm rejects invalid form selector', async () => {
    await b.goto(FORM_HTML);
    await assert.rejects(
      () => b.fillForm('#nonexistent', '{"x":"y"}'),
      /not found/i,
    );
  });

  it('autoFillForm skips unmatched fields', async () => {
    await b.goto(FORM_HTML);
    const profile = JSON.stringify({ zzz: 'nope' });
    const result = JSON.parse(await b.autoFillForm('#signup', profile));
    assert.equal(result.filled, 0);
    assert.ok(result.skipped > 0);
  });

  it('fillForm with invalid JSON rejects', async () => {
    await b.goto(FORM_HTML);
    await assert.rejects(() => b.fillForm('#signup', 'not-json'), /reason/i);
  });
});

describe('Sitemap', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('generateSitemap produces valid XML', () => {
    const entries = JSON.stringify([
      { url: 'https://example.com/', priority: 1.0 },
      { url: 'https://example.com/about', changefreq: 'monthly' },
    ]);
    const xml = b.generateSitemap(entries);
    assert.ok(xml.includes('<?xml'));
    assert.ok(xml.includes('<urlset'));
    assert.ok(xml.includes('https://example.com/'));
    assert.ok(xml.includes('https://example.com/about'));
  });

  it('generateSitemapIndex produces index XML', () => {
    const urls = JSON.stringify([
      'https://example.com/sitemap1.xml',
      'https://example.com/sitemap2.xml',
    ]);
    const xml = b.generateSitemapIndex(urls);
    assert.ok(xml.includes('<sitemapindex'));
    assert.ok(xml.includes('sitemap1.xml'));
  });

  it('saveSitemap writes to file', () => {
    const entries = JSON.stringify([
      { url: 'https://example.com/' },
    ]);
    const tmp = path.join(os.tmpdir(), `sitemap-${Date.now()}.xml`);
    const count = b.saveSitemap(entries, tmp);
    assert.equal(count, 1);
    const xml = fs.readFileSync(tmp, 'utf-8');
    assert.ok(xml.includes('<urlset'));
    fs.unlinkSync(tmp);
  });

  it('parseSitemap parses XML back to entries', () => {
    const entries = JSON.parse(b.parseSitemap(SITEMAP_XML));
    assert.equal(entries.length, 2);
    assert.equal(entries[0].url, 'https://example.com/');
    assert.equal(entries[0].priority, 1.0);
    assert.equal(entries[1].changefreq, 'monthly');
  });

  it('sitemapFromCrawl converts crawl results', () => {
    const results = JSON.stringify([
      { url: 'https://a.com/', status: 'success', title: 'A', depth: 0, links_found: 2, content: null, error: null, duration_ms: 100, timestamp: 0 },
      { url: 'https://a.com/fail', status: 'error', title: '', depth: 1, links_found: 0, content: null, error: 'fail', duration_ms: 50, timestamp: 0 },
    ]);
    const entries = JSON.parse(b.sitemapFromCrawl(results));
    assert.equal(entries.length, 1);
    assert.equal(entries[0].url, 'https://a.com/');
  });

  it('generateSitemap with custom config', () => {
    const entries = JSON.stringify([{ url: 'https://x.com/' }]);
    const config = JSON.stringify({ base_url: '', default_changefreq: 'daily', default_priority: 0.8, include_lastmod: false });
    const xml = b.generateSitemap(entries, config);
    assert.ok(xml.includes('<changefreq>daily</changefreq>'));
    assert.ok(xml.includes('<priority>0.8</priority>'));
  });

  it('parseSitemap on empty XML returns empty', () => {
    const entries = JSON.parse(b.parseSitemap('<urlset></urlset>'));
    assert.equal(entries.length, 0);
  });

  it('generateSitemap with invalid entries JSON rejects', () => {
    assert.throws(() => b.generateSitemap('not-json'), /reason/i);
  });
});
