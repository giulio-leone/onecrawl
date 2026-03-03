import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const ROBOTS_TXT = `
# Example robots.txt
User-agent: Googlebot
Allow: /public/
Disallow: /private/
Crawl-delay: 2

User-agent: *
Disallow: /admin/
Disallow: /tmp/

Sitemap: https://example.com/sitemap.xml
Sitemap: https://example.com/sitemap2.xml
`;

describe('Robots.txt', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('robotsParse parses rules and sitemaps', () => {
    const raw = b.robotsParse(ROBOTS_TXT);
    const robots = JSON.parse(raw);
    assert.ok(Array.isArray(robots.rules));
    assert.ok(robots.rules.length >= 2);
    assert.deepEqual(robots.sitemaps, [
      'https://example.com/sitemap.xml',
      'https://example.com/sitemap2.xml',
    ]);
  });

  it('robotsIsAllowed allows permitted paths', () => {
    const robots = b.robotsParse(ROBOTS_TXT);
    assert.equal(b.robotsIsAllowed(robots, 'Googlebot', '/public/page'), true);
    assert.equal(b.robotsIsAllowed(robots, 'Googlebot', '/private/secret'), false);
  });

  it('robotsIsAllowed respects wildcard agent', () => {
    const robots = b.robotsParse(ROBOTS_TXT);
    assert.equal(b.robotsIsAllowed(robots, 'RandomBot', '/admin/settings'), false);
    assert.equal(b.robotsIsAllowed(robots, 'RandomBot', '/about'), true);
  });

  it('robotsCrawlDelay returns delay for matching agent', () => {
    const robots = b.robotsParse(ROBOTS_TXT);
    const delay = b.robotsCrawlDelay(robots, 'Googlebot');
    assert.equal(delay, 2);
  });

  it('robotsCrawlDelay returns null for no delay', () => {
    const robots = b.robotsParse(ROBOTS_TXT);
    const delay = b.robotsCrawlDelay(robots, 'RandomBot');
    assert.equal(delay, null);
  });

  it('robotsSitemaps returns declared sitemaps', () => {
    const robots = b.robotsParse(ROBOTS_TXT);
    const sitemaps = JSON.parse(b.robotsSitemaps(robots));
    assert.ok(Array.isArray(sitemaps));
    assert.equal(sitemaps.length, 2);
    assert.ok(sitemaps[0].includes('sitemap.xml'));
  });

  it('robotsParse handles empty content', () => {
    const raw = b.robotsParse('');
    const robots = JSON.parse(raw);
    assert.deepEqual(robots.rules, []);
    assert.deepEqual(robots.sitemaps, []);
  });

  it('robotsParse handles comments-only content', () => {
    const raw = b.robotsParse('# just a comment\n# another comment');
    const robots = JSON.parse(raw);
    assert.deepEqual(robots.rules, []);
  });
});
