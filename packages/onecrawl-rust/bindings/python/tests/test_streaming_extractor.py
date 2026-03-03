"""Tests for the Streaming Extractor module."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser

ITEMS_HTML = """data:text/html,
<html><body>
  <div class="item"><h2>Alpha</h2><span class="price">$10</span><a href="https://a.com">Link</a></div>
  <div class="item"><h2>Beta</h2><span class="price">$20</span><a href="https://b.com">Link</a></div>
  <div class="item"><h2>Gamma</h2><span class="price">$30</span><a href="https://c.com">Link</a></div>
</body></html>"""

SINGLE_HTML = """data:text/html,
<html><head><title>Profile</title></head><body>
  <h1>John Doe</h1>
  <span class="role">Engineer</span>
</body></html>"""


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestStreamingExtractor:
    def test_extract_items(self, browser):
        browser.goto(ITEMS_HTML)
        schema = json.dumps({
            "item_selector": ".item",
            "fields": [
                {"name": "title", "selector": "h2", "extract": "text", "transform": "trim", "required": True},
                {"name": "price", "selector": ".price", "extract": "text", "transform": None, "required": False},
            ],
            "pagination": None,
        })
        result = json.loads(browser.extract_items(schema))
        assert result["total_items"] == 3
        assert len(result["items"]) == 3
        assert result["items"][0]["fields"]["title"] == "Alpha"
        assert result["items"][1]["fields"]["price"] == "$20"

    def test_extract_href(self, browser):
        browser.goto(ITEMS_HTML)
        schema = json.dumps({
            "item_selector": ".item",
            "fields": [
                {"name": "link", "selector": "a", "extract": "href", "transform": None, "required": False},
            ],
            "pagination": None,
        })
        result = json.loads(browser.extract_items(schema))
        assert result["items"][0]["fields"]["link"] == "https://a.com/"

    def test_extract_with_pagination_no_config(self, browser):
        browser.goto(ITEMS_HTML)
        schema = json.dumps({
            "item_selector": ".item",
            "fields": [
                {"name": "title", "selector": "h2", "extract": "text", "transform": None, "required": False},
            ],
            "pagination": None,
        })
        result = json.loads(browser.extract_with_pagination(schema))
        assert result["total_items"] == 3

    def test_extract_single(self, browser):
        browser.goto(SINGLE_HTML)
        rules = json.dumps([
            {"name": "name", "selector": "h1", "extract": "text", "transform": "trim", "required": True},
            {"name": "role", "selector": ".role", "extract": "text", "transform": "uppercase", "required": False},
        ])
        result = json.loads(browser.extract_single(rules))
        assert result["name"] == "John Doe"
        assert result["role"] == "ENGINEER"

    def test_extract_html(self, browser):
        browser.goto(ITEMS_HTML)
        schema = json.dumps({
            "item_selector": ".item",
            "fields": [
                {"name": "content", "selector": "h2", "extract": "html", "transform": None, "required": False},
            ],
            "pagination": None,
        })
        result = json.loads(browser.extract_items(schema))
        assert result["items"][0]["fields"]["content"] == "Alpha"

    def test_export_csv(self, browser):
        items = json.dumps([
            {"index": 0, "page": 1, "fields": {"name": "Alice", "age": "30"}},
            {"index": 1, "page": 1, "fields": {"name": "Bob", "age": "25"}},
        ])
        with tempfile.NamedTemporaryFile(suffix=".csv", delete=False) as f:
            tmp_path = f.name
        try:
            count = browser.export_csv(items, tmp_path)
            assert count == 2
            csv_content = open(tmp_path).read()
            assert "Alice" in csv_content
            assert "Bob" in csv_content
        finally:
            os.unlink(tmp_path)

    def test_export_json(self, browser):
        items = json.dumps([
            {"index": 0, "page": 1, "fields": {"x": "1"}},
        ])
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp_path = f.name
        try:
            count = browser.export_json_file(items, tmp_path)
            assert count == 1
            data = json.loads(open(tmp_path).read())
            assert isinstance(data, list)
            assert data[0]["fields"]["x"] == "1"
        finally:
            os.unlink(tmp_path)

    def test_extract_empty(self, browser):
        browser.goto("data:text/html,<html><body><p>Nothing</p></body></html>")
        schema = json.dumps({
            "item_selector": ".nonexistent",
            "fields": [
                {"name": "x", "selector": "span", "extract": "text", "transform": None, "required": False},
            ],
            "pagination": None,
        })
        result = json.loads(browser.extract_items(schema))
        assert result["total_items"] == 0
        assert len(result["items"]) == 0
