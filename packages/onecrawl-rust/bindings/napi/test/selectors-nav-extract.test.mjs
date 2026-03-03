import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const HTML = `data:text/html,
<html lang="en">
<head><title>Selectors Test</title><meta name="description" content="test page"></head>
<body>
  <div id="main" class="container">
    <h1>Hello World</h1>
    <p class="intro">First paragraph</p>
    <p class="intro">Second paragraph with <a href="https://example.com" rel="nofollow">link</a></p>
    <ul>
      <li class="item">Item 1</li>
      <li class="item">Item 2</li>
      <li class="item">Item 3</li>
    </ul>
    <div class="footer">
      <span>Footer text</span>
    </div>
  </div>
</body>
</html>`;

describe('Smart Selectors', () => {
  let b;
  before(async () => {
    b = await NativeBrowser.launch(true);
    await b.goto(HTML);
  });
  after(async () => { if (b) await b.close(); });

  it('cssSelect finds elements by selector', async () => {
    const raw = await b.cssSelect('p.intro');
    const result = JSON.parse(raw);
    assert.equal(result.count, 2);
    assert.equal(result.results[0].tag, 'p');
    assert.ok(result.results[0].text.includes('First paragraph'));
  });

  it('cssSelect with ::text pseudo-element', async () => {
    const raw = await b.cssSelect('h1::text');
    const result = JSON.parse(raw);
    assert.equal(result.count, 1);
    assert.ok(result.results[0].html.includes('Hello World'));
  });

  it('cssSelect with ::attr(href) pseudo-element', async () => {
    const raw = await b.cssSelect('a::attr(href)');
    const result = JSON.parse(raw);
    assert.equal(result.count, 1);
    assert.equal(result.results[0].text, 'https://example.com');
  });

  it('xpathSelect finds elements', async () => {
    const raw = await b.xpathSelect('//li');
    const result = JSON.parse(raw);
    assert.equal(result.count, 3);
    assert.equal(result.results[0].tag, 'li');
  });

  it('findByText finds elements containing text', async () => {
    const raw = await b.findByText('Item 2');
    const result = JSON.parse(raw);
    assert.ok(result.count >= 1);
    assert.ok(result.results.some(el => el.tag === 'li'));
  });

  it('findByRegex matches pattern', async () => {
    const raw = await b.findByRegex('Item \\d+', 'li');
    const result = JSON.parse(raw);
    assert.equal(result.count, 3);
  });

  it('autoSelector generates a selector', async () => {
    const selector = await b.autoSelector('#main');
    assert.ok(selector.includes('main') || selector.includes('#main'));
  });
});

describe('DOM Navigation', () => {
  let b;
  before(async () => {
    b = await NativeBrowser.launch(true);
    await b.goto(HTML);
  });
  after(async () => { if (b) await b.close(); });

  it('getParent returns parent element', async () => {
    const raw = await b.getParent('h1');
    assert.ok(raw);
    const el = JSON.parse(raw);
    assert.equal(el.tag, 'div');
  });

  it('getChildren returns child elements', async () => {
    const raw = await b.getChildren('ul');
    const children = JSON.parse(raw);
    assert.equal(children.length, 3);
    assert.equal(children[0].tag, 'li');
  });

  it('getNextSibling returns next sibling', async () => {
    const raw = await b.getNextSibling('h1');
    assert.ok(raw);
    const el = JSON.parse(raw);
    assert.equal(el.tag, 'p');
  });

  it('getPrevSibling returns previous sibling', async () => {
    const raw = await b.getPrevSibling('ul');
    assert.ok(raw);
    const el = JSON.parse(raw);
    assert.equal(el.tag, 'p');
  });

  it('getSiblings returns all siblings', async () => {
    const raw = await b.getSiblings('h1');
    const siblings = JSON.parse(raw);
    assert.ok(siblings.length >= 3);
  });

  it('findSimilar finds similar elements', async () => {
    const raw = await b.findSimilar('li.item');
    const similar = JSON.parse(raw);
    assert.ok(similar.length >= 2);
  });

  it('getParent returns null for no-match', async () => {
    const result = await b.getParent('.nonexistent-class-xyz');
    assert.equal(result, null);
  });
});

describe('Content Extraction', () => {
  let b;
  before(async () => {
    b = await NativeBrowser.launch(true);
    await b.goto(HTML);
  });
  after(async () => { if (b) await b.close(); });

  it('extract returns text format', async () => {
    const raw = await b.extract(null, 'text');
    const result = JSON.parse(raw);
    assert.equal(result.format, 'text');
    assert.ok(result.content.includes('Hello World'));
    assert.ok(result.word_count > 0);
  });

  it('extract returns html format', async () => {
    const raw = await b.extract(null, 'html');
    const result = JSON.parse(raw);
    assert.equal(result.format, 'html');
    assert.ok(result.content.includes('<h1>'));
  });

  it('extract returns markdown format', async () => {
    const raw = await b.extract(null, 'markdown');
    const result = JSON.parse(raw);
    assert.equal(result.format, 'markdown');
    assert.ok(result.content.includes('# '));
  });

  it('extract returns json format', async () => {
    const raw = await b.extract(null, 'json');
    const result = JSON.parse(raw);
    assert.equal(result.format, 'json');
    const structured = JSON.parse(result.content);
    assert.ok(Array.isArray(structured.headings));
  });

  it('extract scoped by selector', async () => {
    const raw = await b.extract('ul', 'text');
    const result = JSON.parse(raw);
    assert.ok(result.content.includes('Item 1'));
    assert.ok(!result.content.includes('Hello World'));
  });

  it('extractToFile writes to a file', async () => {
    const tmpFile = path.join(os.tmpdir(), `onecrawl-extract-${Date.now()}.txt`);
    const bytes = await b.extractToFile(tmpFile);
    assert.ok(bytes > 0);
    const content = fs.readFileSync(tmpFile, 'utf8');
    assert.ok(content.includes('Hello World'));
    fs.unlinkSync(tmpFile);
  });

  it('getPageMetadata returns metadata', async () => {
    const raw = await b.getPageMetadata();
    const meta = JSON.parse(raw);
    assert.equal(meta.title, 'Selectors Test');
    assert.equal(meta.description, 'test page');
    assert.equal(meta.language, 'en');
    assert.ok(meta.wordCount > 0);
  });
});
