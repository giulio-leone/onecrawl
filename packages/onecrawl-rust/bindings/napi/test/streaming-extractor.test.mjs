import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const ITEMS_HTML = `data:text/html,
<html><body>
  <div class="item"><h2>Alpha</h2><span class="price">$10</span><a href="https://a.com">Link</a></div>
  <div class="item"><h2>Beta</h2><span class="price">$20</span><a href="https://b.com">Link</a></div>
  <div class="item"><h2>Gamma</h2><span class="price">$30</span><a href="https://c.com">Link</a></div>
</body></html>`;

const SINGLE_HTML = `data:text/html,
<html><head><title>Profile</title></head><body>
  <h1>John Doe</h1>
  <span class="role">Engineer</span>
  <img src="https://img.example.com/photo.jpg" />
</body></html>`;

describe('Streaming Extractor', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('extractItems returns items matching schema', async () => {
    await b.goto(ITEMS_HTML);
    const schema = JSON.stringify({
      item_selector: '.item',
      fields: [
        { name: 'title', selector: 'h2', extract: 'text', transform: 'trim', required: true },
        { name: 'price', selector: '.price', extract: 'text', transform: null, required: false },
      ],
      pagination: null,
    });
    const result = JSON.parse(await b.extractItems(schema));
    assert.equal(result.total_items, 3);
    assert.equal(result.items.length, 3);
    assert.equal(result.items[0].fields.title, 'Alpha');
    assert.equal(result.items[1].fields.price, '$20');
    assert.equal(result.pages_scraped, 1);
  });

  it('extractItems handles href extraction', async () => {
    await b.goto(ITEMS_HTML);
    const schema = JSON.stringify({
      item_selector: '.item',
      fields: [
        { name: 'link', selector: 'a', extract: 'href', transform: null, required: false },
      ],
      pagination: null,
    });
    const result = JSON.parse(await b.extractItems(schema));
    assert.equal(result.items[0].fields.link, 'https://a.com/');
    assert.equal(result.items[2].fields.link, 'https://c.com/');
  });

  it('extractWithPagination works without pagination config', async () => {
    await b.goto(ITEMS_HTML);
    const schema = JSON.stringify({
      item_selector: '.item',
      fields: [
        { name: 'title', selector: 'h2', extract: 'text', transform: null, required: false },
      ],
      pagination: null,
    });
    const result = JSON.parse(await b.extractWithPagination(schema));
    assert.equal(result.total_items, 3);
  });

  it('extractSingle extracts without item_selector', async () => {
    await b.goto(SINGLE_HTML);
    const rules = JSON.stringify([
      { name: 'name', selector: 'h1', extract: 'text', transform: 'trim', required: true },
      { name: 'role', selector: '.role', extract: 'text', transform: 'uppercase', required: false },
    ]);
    const result = JSON.parse(await b.extractSingle(rules));
    assert.equal(result.name, 'John Doe');
    assert.equal(result.role, 'ENGINEER');
  });

  it('extractItems with html extraction', async () => {
    await b.goto(ITEMS_HTML);
    const schema = JSON.stringify({
      item_selector: '.item',
      fields: [
        { name: 'content', selector: 'h2', extract: 'html', transform: null, required: false },
      ],
      pagination: null,
    });
    const result = JSON.parse(await b.extractItems(schema));
    assert.equal(result.items[0].fields.content, 'Alpha');
  });

  it('exportCsv writes CSV file', async () => {
    const items = JSON.stringify([
      { index: 0, page: 1, fields: { name: 'Alice', age: '30' } },
      { index: 1, page: 1, fields: { name: 'Bob', age: '25' } },
    ]);
    const tmpFile = path.join(os.tmpdir(), `onecrawl-test-${Date.now()}.csv`);
    try {
      const count = await b.exportCsv(items, tmpFile);
      assert.equal(count, 2);
      const csv = fs.readFileSync(tmpFile, 'utf8');
      assert.ok(csv.includes('Alice'));
      assert.ok(csv.includes('Bob'));
      assert.ok(csv.includes('age'));
    } finally {
      fs.rmSync(tmpFile, { force: true });
    }
  });

  it('exportJson writes JSON file', async () => {
    const items = JSON.stringify([
      { index: 0, page: 1, fields: { x: '1' } },
    ]);
    const tmpFile = path.join(os.tmpdir(), `onecrawl-test-${Date.now()}.json`);
    try {
      const count = await b.exportJson(items, tmpFile);
      assert.equal(count, 1);
      const json = JSON.parse(fs.readFileSync(tmpFile, 'utf8'));
      assert.ok(Array.isArray(json));
      assert.equal(json[0].fields.x, '1');
    } finally {
      fs.rmSync(tmpFile, { force: true });
    }
  });

  it('extractItems returns empty for no matches', async () => {
    await b.goto('data:text/html,<html><body><p>Nothing</p></body></html>');
    const schema = JSON.stringify({
      item_selector: '.nonexistent',
      fields: [
        { name: 'x', selector: 'span', extract: 'text', transform: null, required: false },
      ],
      pagination: null,
    });
    const result = JSON.parse(await b.extractItems(schema));
    assert.equal(result.total_items, 0);
    assert.equal(result.items.length, 0);
  });
});
