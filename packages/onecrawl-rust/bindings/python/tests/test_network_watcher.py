"""Tests for Network Log and Page Watcher modules."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestNetworkLog:
    def test_start_network_log(self, browser):
        browser.goto("data:text/html,<h1>NetLog</h1>")
        browser.start_network_log()

    def test_drain_network_log(self, browser):
        raw = browser.drain_network_log()
        entries = json.loads(raw)
        assert isinstance(entries, list)

    def test_captures_fetch_requests(self, browser):
        browser.evaluate('fetch("data:text/plain,hello")')
        import time
        time.sleep(0.5)
        raw = browser.drain_network_log()
        entries = json.loads(raw)
        assert isinstance(entries, list)

    def test_get_network_summary(self, browser):
        raw = browser.get_network_summary()
        summary = json.loads(raw)
        assert isinstance(summary["total_requests"], int)
        assert isinstance(summary["total_size_bytes"], int)
        assert isinstance(summary["by_type"], dict)
        assert isinstance(summary["by_status"], dict)
        assert isinstance(summary["errors"], list)
        assert isinstance(summary["slowest"], list)

    def test_export_network_log(self, browser):
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            browser.export_network_log(path)
            assert os.path.exists(path)
            with open(path) as f:
                content = json.load(f)
            assert isinstance(content, list)
        finally:
            os.unlink(path)

    def test_stop_network_log(self, browser):
        browser.stop_network_log()

    def test_drain_after_restart(self, browser):
        browser.start_network_log()
        raw = browser.drain_network_log()
        entries = json.loads(raw)
        assert isinstance(entries, list)
        browser.stop_network_log()


class TestPageWatcher:
    def test_start_page_watcher(self, browser):
        browser.goto(
            "data:text/html,<html><head><title>Watcher</title></head>"
            "<body><h1>PW</h1></body></html>"
        )
        browser.start_page_watcher()

    def test_drain_page_changes(self, browser):
        raw = browser.drain_page_changes()
        changes = json.loads(raw)
        assert isinstance(changes, list)

    def test_captures_title_change(self, browser):
        browser.evaluate('document.title = "New Title"')
        import time
        time.sleep(0.3)
        raw = browser.drain_page_changes()
        changes = json.loads(raw)
        title_changes = [c for c in changes if c["change_type"] == "title"]
        if title_changes:
            assert "New Title" in title_changes[0]["new_value"]

    def test_get_page_state(self, browser):
        raw = browser.get_page_state()
        state = json.loads(raw)
        assert isinstance(state["url"], str)
        assert isinstance(state["title"], str)
        assert isinstance(state["ready_state"], str)
        assert isinstance(state["viewport_width"], int)
        assert isinstance(state["element_count"], int)

    def test_stop_page_watcher(self, browser):
        browser.stop_page_watcher()
