import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

const HTML = `data:text/html,
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Structured Data Test</title>
  <meta name="description" content="A test page for structured data extraction">
  <meta name="author" content="Test Author">
  <meta name="keywords" content="test, structured, data">
  <meta name="robots" content="index, follow">
  <link rel="canonical" href="https://example.com/test">
  <link rel="icon" href="/favicon.ico">
  <meta property="og:title" content="OG Title">
  <meta property="og:description" content="OG Description">
  <meta property="og:image" content="https://example.com/image.jpg">
  <meta property="og:url" content="https://example.com/test">
  <meta property="og:site_name" content="TestSite">
  <meta property="og:type" content="article">
  <meta property="og:locale" content="en_US">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="Twitter Title">
  <meta name="twitter:description" content="Twitter Desc">
  <meta name="twitter:image" content="https://example.com/twitter.jpg">
  <meta name="twitter:site" content="@testsite">
  <meta name="twitter:creator" content="@testauthor">
  <script type="application/ld+json">
  {
    "@context": "https://schema.org",
    "@type": "Article",
    "headline": "Test Article",
    "author": {"@type": "Person", "name": "Test Author"}
  }
  </script>
</head>
<body><h1>Hello</h1></body>
</html>`;

describe('Structured Data Extractor', () => {
  let b;
  before(async () => {
    b = await NativeBrowser.launch(true);
    await b.goto(HTML);
  });
  after(async () => { if (b) await b.close(); });

  it('extract all structured data', async () => {
    const raw = await b.structuredExtractAll();
    const data = JSON.parse(raw);
    assert.ok(data.json_ld.length >= 1);
    assert.ok(data.open_graph);
    assert.ok(data.twitter_card);
    assert.ok(data.metadata);
    assert.ok(data.schema_types.includes('Article'));
  });

  it('extract JSON-LD', async () => {
    const raw = await b.structuredJsonLd();
    const ld = JSON.parse(raw);
    assert.ok(Array.isArray(ld));
    assert.equal(ld[0].data_type, 'Article');
    assert.ok(ld[0].context.includes('schema.org'));
  });

  it('extract OpenGraph', async () => {
    const raw = await b.structuredOpenGraph();
    const og = JSON.parse(raw);
    assert.equal(og.title, 'OG Title');
    assert.equal(og.description, 'OG Description');
    assert.equal(og.image, 'https://example.com/image.jpg');
    assert.equal(og.site_name, 'TestSite');
    assert.equal(og.og_type, 'article');
  });

  it('extract Twitter Card', async () => {
    const raw = await b.structuredTwitterCard();
    const tc = JSON.parse(raw);
    assert.equal(tc.card, 'summary_large_image');
    assert.equal(tc.title, 'Twitter Title');
    assert.equal(tc.site, '@testsite');
    assert.equal(tc.creator, '@testauthor');
  });

  it('extract page metadata', async () => {
    const raw = await b.structuredMetadata();
    const meta = JSON.parse(raw);
    assert.equal(meta.title, 'Structured Data Test');
    assert.equal(meta.description, 'A test page for structured data extraction');
    assert.equal(meta.author, 'Test Author');
    assert.equal(meta.canonical_url, 'https://example.com/test');
    assert.ok(meta.keywords.includes('test'));
    assert.ok(meta.keywords.includes('structured'));
  });

  it('validate structured data — complete page', async () => {
    const all = await b.structuredExtractAll();
    const raw = b.structuredValidate(all);
    const warnings = JSON.parse(raw);
    // Complete page should have very few warnings
    assert.ok(Array.isArray(warnings));
  });

  it('validate structured data — incomplete page', () => {
    const emptyData = JSON.stringify({
      json_ld: [],
      open_graph: null,
      twitter_card: null,
      metadata: {
        title: '',
        description: '',
        canonical_url: null,
        author: null,
        published_date: null,
        modified_date: null,
        language: null,
        charset: null,
        favicon: null,
        robots: null,
        keywords: [],
      },
      schema_types: [],
    });
    const raw = b.structuredValidate(emptyData);
    const warnings = JSON.parse(raw);
    assert.ok(warnings.length >= 4);
    assert.ok(warnings.some(w => w.includes('JSON-LD')));
    assert.ok(warnings.some(w => w.includes('OpenGraph')));
    assert.ok(warnings.some(w => w.includes('title')));
  });

  it('extract from page with no structured data', async () => {
    await b.goto('data:text/html,<html><head><title>Plain</title></head><body>No data</body></html>');
    const raw = await b.structuredExtractAll();
    const data = JSON.parse(raw);
    assert.equal(data.json_ld.length, 0);
    assert.equal(data.open_graph, null);
    assert.equal(data.twitter_card, null);
    assert.equal(data.metadata.title, 'Plain');
  });
});
