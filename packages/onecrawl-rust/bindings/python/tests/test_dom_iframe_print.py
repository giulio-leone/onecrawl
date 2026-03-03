"""Tests for DOM Observer, Iframe management, and enhanced Print/PDF."""

import json
import time
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestDomObserver:
    def test_start_dom_observer(self, browser):
        browser.goto("data:text/html,<h1>DOM</h1>")
        browser.start_dom_observer()

    def test_drain_dom_mutations(self, browser):
        browser.evaluate('document.body.innerHTML += "<p>new</p>"')
        raw = browser.drain_dom_mutations()
        mutations = json.loads(raw)
        assert isinstance(mutations, list)

    def test_captures_child_list_mutations(self, browser):
        browser.evaluate('document.body.appendChild(document.createElement("span"))')
        raw = browser.drain_dom_mutations()
        mutations = json.loads(raw)
        child_list = [m for m in mutations if m["mutation_type"] == "childList"]
        assert len(child_list) > 0

    def test_stop_dom_observer(self, browser):
        browser.stop_dom_observer()

    def test_get_dom_snapshot(self, browser):
        html = browser.get_dom_snapshot()
        assert "<" in html
        assert len(html) > 0

    def test_get_dom_snapshot_with_selector(self, browser):
        browser.goto('data:text/html,<div id="target">content</div>')
        html = browser.get_dom_snapshot("#target")
        assert "content" in html


class TestIframe:
    def test_list_iframes(self, browser):
        browser.goto('data:text/html,<iframe src="about:blank"></iframe>')
        raw = browser.list_iframes()
        iframes = json.loads(raw)
        assert isinstance(iframes, list)
        assert len(iframes) >= 1

    def test_list_iframes_empty(self, browser):
        browser.goto("data:text/html,<h1>No Frames</h1>")
        raw = browser.list_iframes()
        iframes = json.loads(raw)
        assert isinstance(iframes, list)
        assert len(iframes) == 0

    def test_get_iframe_content(self, browser):
        browser.goto('data:text/html,<iframe srcdoc="<p>hello</p>"></iframe>')
        time.sleep(0.5)
        content = browser.get_iframe_content(0)
        assert isinstance(content, str)

    def test_eval_in_iframe(self, browser):
        browser.goto('data:text/html,<iframe srcdoc="<p>eval</p>"></iframe>')
        time.sleep(0.5)
        raw = browser.eval_in_iframe(0, "1 + 1")
        assert isinstance(raw, str)


class TestPrint:
    def test_print_to_pdf(self, browser):
        browser.goto("data:text/html,<h1>PDF Test</h1>")
        data = browser.print_to_pdf()
        assert isinstance(data, bytes)
        assert len(data) > 0

    def test_print_to_pdf_with_options(self, browser):
        opts = json.dumps({"landscape": True, "print_background": True})
        data = browser.print_to_pdf(opts)
        assert len(data) > 0

    def test_get_print_metrics(self, browser):
        raw = browser.get_print_metrics()
        metrics = json.loads(raw)
        assert isinstance(metrics, dict)
        assert "width" in metrics
        assert "height" in metrics
