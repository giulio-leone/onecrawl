"""Tests for the Form Filler and Sitemap modules."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser

FORM_HTML = """data:text/html,
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
</body></html>"""

SITEMAP_XML = """<?xml version="1.0" encoding="UTF-8"?>
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
</urlset>"""


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestFormFiller:
    def test_detect_forms(self, browser):
        browser.goto(FORM_HTML)
        forms = json.loads(browser.detect_forms())
        assert isinstance(forms, list)
        assert len(forms) == 1
        assert forms[0]["method"] == "POST"
        assert len(forms[0]["fields"]) >= 5

    def test_fill_form(self, browser):
        browser.goto(FORM_HTML)
        values = json.dumps({"email": "test@x.com", "name": "Alice"})
        result = json.loads(browser.fill_form("#signup", values))
        assert result["filled"] >= 2
        assert result["errors"] == []

    def test_auto_fill_form(self, browser):
        browser.goto(FORM_HTML)
        profile = json.dumps({"email": "a@b.com", "name": "Bob", "phone": "123"})
        result = json.loads(browser.auto_fill_form("#signup", profile))
        assert result["filled"] >= 2

    def test_submit_form(self, browser):
        browser.goto(FORM_HTML)
        browser.submit_form("#signup")

    def test_detect_forms_empty_page(self, browser):
        browser.goto("data:text/html,<html><body><p>No forms</p></body></html>")
        forms = json.loads(browser.detect_forms())
        assert len(forms) == 0

    def test_fill_form_invalid_selector(self, browser):
        browser.goto(FORM_HTML)
        with pytest.raises(Exception, match="not found"):
            browser.fill_form("#nonexistent", '{"x":"y"}')

    def test_auto_fill_skips_unmatched(self, browser):
        browser.goto(FORM_HTML)
        result = json.loads(browser.auto_fill_form("#signup", '{"zzz":"nope"}'))
        assert result["filled"] == 0
        assert result["skipped"] > 0

    def test_fill_form_invalid_json(self, browser):
        browser.goto(FORM_HTML)
        with pytest.raises(Exception):
            browser.fill_form("#signup", "not-json")


class TestSitemap:
    def test_generate_sitemap(self, browser):
        entries = json.dumps([
            {"url": "https://example.com/", "priority": 1.0},
            {"url": "https://example.com/about", "changefreq": "monthly"},
        ])
        xml = browser.generate_sitemap(entries)
        assert "<?xml" in xml
        assert "<urlset" in xml
        assert "https://example.com/" in xml

    def test_generate_sitemap_index(self, browser):
        urls = json.dumps([
            "https://example.com/sitemap1.xml",
            "https://example.com/sitemap2.xml",
        ])
        xml = browser.generate_sitemap_index(urls)
        assert "<sitemapindex" in xml
        assert "sitemap1.xml" in xml

    def test_save_sitemap(self, browser):
        entries = json.dumps([{"url": "https://example.com/"}])
        with tempfile.NamedTemporaryFile(suffix=".xml", delete=False) as f:
            tmp = f.name
        try:
            count = browser.save_sitemap(entries, tmp)
            assert count == 1
            xml = open(tmp).read()
            assert "<urlset" in xml
        finally:
            os.unlink(tmp)

    def test_parse_sitemap(self, browser):
        entries = json.loads(browser.parse_sitemap(SITEMAP_XML))
        assert len(entries) == 2
        assert entries[0]["url"] == "https://example.com/"
        assert entries[0]["priority"] == 1.0
        assert entries[1]["changefreq"] == "monthly"

    def test_sitemap_from_crawl(self, browser):
        results = json.dumps([
            {"url": "https://a.com/", "status": "success", "title": "A", "depth": 0, "links_found": 2, "content": None, "error": None, "duration_ms": 100, "timestamp": 0},
            {"url": "https://a.com/fail", "status": "error", "title": "", "depth": 1, "links_found": 0, "content": None, "error": "fail", "duration_ms": 50, "timestamp": 0},
        ])
        entries = json.loads(browser.sitemap_from_crawl(results))
        assert len(entries) == 1
        assert entries[0]["url"] == "https://a.com/"

    def test_generate_with_custom_config(self, browser):
        entries = json.dumps([{"url": "https://x.com/"}])
        config = json.dumps({"base_url": "", "default_changefreq": "daily", "default_priority": 0.8, "include_lastmod": False})
        xml = browser.generate_sitemap(entries, config)
        assert "<changefreq>daily</changefreq>" in xml
        assert "<priority>0.8</priority>" in xml

    def test_parse_empty_sitemap(self, browser):
        entries = json.loads(browser.parse_sitemap("<urlset></urlset>"))
        assert len(entries) == 0

    def test_generate_invalid_json(self, browser):
        with pytest.raises(Exception):
            browser.generate_sitemap("not-json")
