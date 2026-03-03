"""Tests for smart selectors, DOM navigation, and content extraction modules."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser

HTML = """data:text/html,
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
</html>"""


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    b.goto(HTML)
    yield b
    b.close()


class TestSmartSelectors:
    def test_css_select(self, browser):
        raw = browser.css_select("p.intro")
        result = json.loads(raw)
        assert result["count"] == 2
        assert result["results"][0]["tag"] == "p"
        assert "First paragraph" in result["results"][0]["text"]

    def test_css_select_pseudo_text(self, browser):
        raw = browser.css_select("h1::text")
        result = json.loads(raw)
        assert result["count"] == 1
        assert "Hello World" in result["results"][0]["html"]

    def test_css_select_pseudo_attr(self, browser):
        raw = browser.css_select("a::attr(href)")
        result = json.loads(raw)
        assert result["count"] == 1
        assert result["results"][0]["text"] == "https://example.com"

    def test_xpath_select(self, browser):
        raw = browser.xpath_select("//li")
        result = json.loads(raw)
        assert result["count"] == 3
        assert result["results"][0]["tag"] == "li"

    def test_find_by_text(self, browser):
        raw = browser.find_by_text("Item 2")
        result = json.loads(raw)
        assert result["count"] >= 1
        assert any(el["tag"] == "li" for el in result["results"])

    def test_find_by_regex(self, browser):
        raw = browser.find_by_regex("Item \\d+", tag="li")
        result = json.loads(raw)
        assert result["count"] == 3

    def test_auto_selector(self, browser):
        selector = browser.auto_selector("#main")
        assert "main" in selector or "#main" in selector


class TestDomNavigation:
    def test_get_parent(self, browser):
        raw = browser.get_parent("h1")
        assert raw is not None
        el = json.loads(raw)
        assert el["tag"] == "div"

    def test_get_children(self, browser):
        raw = browser.get_children("ul")
        children = json.loads(raw)
        assert len(children) == 3
        assert children[0]["tag"] == "li"

    def test_get_next_sibling(self, browser):
        raw = browser.get_next_sibling("h1")
        assert raw is not None
        el = json.loads(raw)
        assert el["tag"] == "p"

    def test_get_prev_sibling(self, browser):
        raw = browser.get_prev_sibling("ul")
        assert raw is not None
        el = json.loads(raw)
        assert el["tag"] == "p"

    def test_get_siblings(self, browser):
        raw = browser.get_siblings("h1")
        siblings = json.loads(raw)
        assert len(siblings) >= 3

    def test_find_similar(self, browser):
        raw = browser.find_similar("li.item")
        similar = json.loads(raw)
        assert len(similar) >= 2

    def test_get_parent_no_match(self, browser):
        result = browser.get_parent(".nonexistent-class-xyz")
        assert result is None


class TestContentExtraction:
    def test_extract_text(self, browser):
        raw = browser.extract_content(format="text")
        result = json.loads(raw)
        assert result["format"] == "text"
        assert "Hello World" in result["content"]
        assert result["word_count"] > 0

    def test_extract_html(self, browser):
        raw = browser.extract_content(format="html")
        result = json.loads(raw)
        assert result["format"] == "html"
        assert "<h1>" in result["content"]

    def test_extract_markdown(self, browser):
        raw = browser.extract_content(format="markdown")
        result = json.loads(raw)
        assert result["format"] == "markdown"
        assert "# " in result["content"]

    def test_extract_json(self, browser):
        raw = browser.extract_content(format="json")
        result = json.loads(raw)
        assert result["format"] == "json"
        structured = json.loads(result["content"])
        assert isinstance(structured["headings"], list)

    def test_extract_scoped(self, browser):
        raw = browser.extract_content(selector="ul", format="text")
        result = json.loads(raw)
        assert "Item 1" in result["content"]
        assert "Hello World" not in result["content"]

    def test_extract_to_file(self, browser):
        with tempfile.NamedTemporaryFile(suffix=".txt", delete=False) as f:
            tmp_path = f.name
        try:
            nbytes = browser.extract_to_file(tmp_path)
            assert nbytes > 0
            content = open(tmp_path).read()
            assert "Hello World" in content
        finally:
            os.unlink(tmp_path)

    def test_get_page_metadata(self, browser):
        raw = browser.get_page_metadata()
        meta = json.loads(raw)
        assert meta["title"] == "Selectors Test"
        assert meta["description"] == "test page"
        assert meta["language"] == "en"
        assert meta["wordCount"] > 0
