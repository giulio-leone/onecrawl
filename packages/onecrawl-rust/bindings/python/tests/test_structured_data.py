"""Tests for the Structured Data Extractor module."""

import json
import pytest
from onecrawl import Browser


HTML = """data:text/html,
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
</html>"""


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    b.goto(HTML)
    yield b
    b.close()


class TestStructuredData:
    def test_extract_all(self, browser):
        raw = browser.structured_extract_all()
        data = json.loads(raw)
        assert len(data["json_ld"]) >= 1
        assert data["open_graph"] is not None
        assert data["twitter_card"] is not None
        assert data["metadata"] is not None
        assert "Article" in data["schema_types"]

    def test_extract_json_ld(self, browser):
        raw = browser.structured_json_ld()
        ld = json.loads(raw)
        assert isinstance(ld, list)
        assert ld[0]["data_type"] == "Article"
        assert "schema.org" in ld[0]["context"]

    def test_extract_open_graph(self, browser):
        raw = browser.structured_open_graph()
        og = json.loads(raw)
        assert og["title"] == "OG Title"
        assert og["description"] == "OG Description"
        assert og["image"] == "https://example.com/image.jpg"
        assert og["site_name"] == "TestSite"

    def test_extract_twitter_card(self, browser):
        raw = browser.structured_twitter_card()
        tc = json.loads(raw)
        assert tc["card"] == "summary_large_image"
        assert tc["title"] == "Twitter Title"
        assert tc["site"] == "@testsite"
        assert tc["creator"] == "@testauthor"

    def test_extract_metadata(self, browser):
        raw = browser.structured_metadata()
        meta = json.loads(raw)
        assert meta["title"] == "Structured Data Test"
        assert meta["description"] == "A test page for structured data extraction"
        assert meta["author"] == "Test Author"
        assert meta["canonical_url"] == "https://example.com/test"
        assert "test" in meta["keywords"]
        assert "structured" in meta["keywords"]

    def test_validate_complete_page(self, browser):
        all_data = browser.structured_extract_all()
        raw = browser.structured_validate(all_data)
        warnings = json.loads(raw)
        assert isinstance(warnings, list)

    def test_validate_incomplete_data(self, browser):
        empty_data = json.dumps({
            "json_ld": [],
            "open_graph": None,
            "twitter_card": None,
            "metadata": {
                "title": "",
                "description": "",
                "canonical_url": None,
                "author": None,
                "published_date": None,
                "modified_date": None,
                "language": None,
                "charset": None,
                "favicon": None,
                "robots": None,
                "keywords": [],
            },
            "schema_types": [],
        })
        raw = browser.structured_validate(empty_data)
        warnings = json.loads(raw)
        assert len(warnings) >= 4
        assert any("JSON-LD" in w for w in warnings)
        assert any("OpenGraph" in w for w in warnings)

    def test_extract_from_plain_page(self, browser):
        browser.goto("data:text/html,<html><head><title>Plain</title></head><body>No data</body></html>")
        raw = browser.structured_extract_all()
        data = json.loads(raw)
        assert len(data["json_ld"]) == 0
        assert data["open_graph"] is None
        assert data["twitter_card"] is None
        assert data["metadata"]["title"] == "Plain"
        # Navigate back for other tests
        browser.goto(HTML)
