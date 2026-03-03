"""Tests for HAR recording, WebSocket interception, and code coverage."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestHarRecording:
    def test_start_har_recording(self, browser):
        browser.goto("data:text/html,<h1>HAR</h1>")
        browser.start_har_recording()

    def test_drain_har_entries(self, browser):
        count = browser.drain_har_entries()
        assert isinstance(count, int)

    def test_export_har(self, browser):
        raw = browser.export_har()
        har = json.loads(raw)
        assert har["log"]["version"] == "1.2"
        assert har["log"]["creator"]["name"] == "OneCrawl"
        assert isinstance(har["log"]["entries"], list)


class TestWebSocketRecording:
    def test_start_ws_recording(self, browser):
        browser.goto("data:text/html,<h1>WS</h1>")
        browser.start_ws_recording()

    def test_drain_ws_frames(self, browser):
        count = browser.drain_ws_frames()
        assert count == 0

    def test_export_ws_frames(self, browser):
        raw = browser.export_ws_frames()
        frames = json.loads(raw)
        assert isinstance(frames, list)

    def test_active_ws_connections(self, browser):
        count = browser.active_ws_connections()
        assert count == 0


class TestJsCoverage:
    def test_start_js_coverage(self, browser):
        browser.goto("data:text/html,<script>function f(){return 1;} f();</script>")
        browser.start_js_coverage()

    def test_stop_js_coverage(self, browser):
        browser.evaluate("(() => { let x = 1; return x + 1; })()")
        raw = browser.stop_js_coverage()
        report = json.loads(raw)
        assert "scripts" in report
        assert "total_bytes" in report
        assert "used_bytes" in report
        assert "overall_percent" in report


class TestCssCoverage:
    def test_start_css_coverage(self, browser):
        browser.goto("data:text/html,<style>body{color:red;}</style><p>Hi</p>")
        browser.start_css_coverage()

    def test_get_css_coverage(self, browser):
        raw = browser.get_css_coverage()
        report = json.loads(raw)
        assert "used_properties" in report
        assert "total_stylesheets" in report
