"""Tests for the Spider / Crawl module."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser

HTML = """data:text/html,
<html>
<head><title>Spider Root</title></head>
<body>
  <h1>Home</h1>
  <a href="https://example.com/page1">Page 1</a>
  <p class="content">Root content text</p>
</body>
</html>"""


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestSpiderCrawl:
    def test_crawl_returns_results(self, browser):
        config = json.dumps({
            "start_urls": ["data:text/html,<html><head><title>T</title></head><body>Hello</body></html>"],
            "max_depth": 0,
            "max_pages": 1,
            "follow_links": False,
            "same_domain_only": False,
        })
        raw = browser.crawl(config)
        results = json.loads(raw)
        assert isinstance(results, list)
        assert len(results) == 1
        assert results[0]["status"] == "success"
        assert results[0]["timestamp"] > 0

    def test_crawl_respects_max_pages(self, browser):
        config = json.dumps({
            "start_urls": [
                "data:text/html,<html><head><title>A</title></head><body>A</body></html>",
                "data:text/html,<html><head><title>B</title></head><body>B</body></html>",
                "data:text/html,<html><head><title>C</title></head><body>C</body></html>",
            ],
            "max_depth": 0,
            "max_pages": 2,
            "follow_links": False,
            "same_domain_only": False,
        })
        results = json.loads(browser.crawl(config))
        assert len(results) <= 2

    def test_crawl_extracts_content(self, browser):
        config = json.dumps({
            "start_urls": ["data:text/html,<html><body><p class='x'>extracted</p></body></html>"],
            "max_depth": 0,
            "max_pages": 1,
            "follow_links": False,
            "same_domain_only": False,
            "extract_selector": "p.x",
            "extract_format": "text",
        })
        results = json.loads(browser.crawl(config))
        assert results[0]["content"] == "extracted"

    def test_crawl_summary(self, browser):
        results = json.dumps([
            {"url": "https://a.com/", "status": "success", "title": "A", "depth": 0, "links_found": 2, "content": None, "error": None, "duration_ms": 100, "timestamp": 0},
            {"url": "https://a.com/x", "status": "error", "title": "", "depth": 1, "links_found": 0, "content": None, "error": "fail", "duration_ms": 50, "timestamp": 0},
        ])
        summary = json.loads(browser.crawl_summary(results))
        assert summary["total_pages"] == 2
        assert summary["successful"] == 1
        assert summary["failed"] == 1
        assert summary["total_links_found"] == 2

    def test_save_load_state_roundtrip(self, browser):
        state = {
            "config": {"start_urls": ["https://example.com"], "max_depth": 3, "max_pages": 100, "concurrency": 3, "delay_ms": 500, "follow_links": True, "same_domain_only": True, "url_patterns": [], "exclude_patterns": [], "extract_selector": None, "extract_format": "text", "timeout_ms": 30000, "user_agent": None},
            "visited": ["https://example.com"],
            "pending": [["https://example.com/a", 1]],
            "results": [],
            "status": "paused",
        }
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp = f.name
        try:
            browser.save_crawl_state(json.dumps(state), tmp)
            loaded = json.loads(browser.load_crawl_state(tmp))
            assert loaded["status"] == "paused"
            assert loaded["visited"] == ["https://example.com"]
        finally:
            os.unlink(tmp)

    def test_export_results_json(self, browser):
        results = [
            {"url": "https://a.com/", "status": "success", "title": "A", "depth": 0, "links_found": 0, "content": None, "error": None, "duration_ms": 10, "timestamp": 0},
        ]
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp = f.name
        try:
            count = browser.export_crawl_results(json.dumps(results), tmp)
            assert count == 1
            data = json.loads(open(tmp).read())
            assert len(data) == 1
        finally:
            os.unlink(tmp)

    def test_export_results_jsonl(self, browser):
        results = [
            {"url": "https://a.com/", "status": "success", "title": "A", "depth": 0, "links_found": 0, "content": None, "error": None, "duration_ms": 10, "timestamp": 0},
            {"url": "https://b.com/", "status": "error", "title": "", "depth": 0, "links_found": 0, "content": None, "error": "e", "duration_ms": 5, "timestamp": 0},
        ]
        with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as f:
            tmp = f.name
        try:
            count = browser.export_crawl_results(json.dumps(results), tmp, "jsonl")
            assert count == 2
            lines = open(tmp).read().strip().split("\n")
            assert len(lines) == 2
            assert json.loads(lines[0])["url"] == "https://a.com/"
        finally:
            os.unlink(tmp)

    def test_crawl_invalid_config_raises(self, browser):
        with pytest.raises(Exception):
            browser.crawl("not-json")
