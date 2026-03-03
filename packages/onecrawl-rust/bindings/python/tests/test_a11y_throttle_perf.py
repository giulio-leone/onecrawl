"""Tests for accessibility audit, network throttling, and performance tracing."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestAccessibility:
    def test_get_accessibility_tree(self, browser):
        browser.goto("data:text/html,<h1>Title</h1><p>Text</p>")
        raw = browser.get_accessibility_tree()
        tree = json.loads(raw)
        assert tree is not None

    def test_get_element_accessibility(self, browser):
        raw = browser.get_element_accessibility("h1")
        info = json.loads(raw)
        assert info is not None

    def test_audit_accessibility(self, browser):
        browser.goto("data:text/html,<img src='x.png'><input type='text'>")
        raw = browser.audit_accessibility()
        audit = json.loads(raw)
        assert "issues" in audit
        assert "summary" in audit


class TestNetworkThrottling:
    def test_set_throttle_slow3g(self, browser):
        browser.set_network_throttle("slow3g")

    def test_set_throttle_custom(self, browser):
        browser.set_network_throttle_custom(1000.0, 500.0, 100.0)

    def test_clear_throttle(self, browser):
        browser.clear_network_throttle()

    def test_unknown_profile_raises(self, browser):
        with pytest.raises(RuntimeError):
            browser.set_network_throttle("unknown")


class TestPerformanceTracing:
    def test_get_performance_metrics(self, browser):
        browser.goto("data:text/html,<h1>Perf</h1>")
        raw = browser.get_performance_metrics()
        metrics = json.loads(raw)
        assert isinstance(metrics, list)

    def test_get_navigation_timing(self, browser):
        raw = browser.get_navigation_timing()
        timing = json.loads(raw)
        assert timing is not None

    def test_get_resource_timing(self, browser):
        raw = browser.get_resource_timing()
        resources = json.loads(raw)
        assert isinstance(resources, list)

    def test_tracing_start_stop(self, browser):
        browser.start_tracing()
        browser.goto("data:text/html,<h1>Trace</h1>")
        raw = browser.stop_tracing()
        trace = json.loads(raw)
        assert trace is not None
