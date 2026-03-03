"""Tests for Rate Limiter and Retry Queue."""

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


class TestRateLimiter:
    def test_stats_returns_valid_json(self, browser):
        raw = browser.rate_limit_stats()
        stats = json.loads(raw)
        assert isinstance(stats, dict)
        assert "total_requests" in stats
        assert "total_throttled" in stats
        assert "status" in stats

    def test_can_proceed_initially(self, browser):
        browser.rate_limit_set(None)
        assert browser.rate_limit_can_proceed() is True

    def test_record_returns_true(self, browser):
        browser.rate_limit_set(None)
        assert browser.rate_limit_record() is True

    def test_wait_zero_when_unlimited(self, browser):
        browser.rate_limit_set("unlimited")
        assert browser.rate_limit_wait() == 0

    def test_set_with_preset(self, browser):
        raw = browser.rate_limit_set("conservative")
        stats = json.loads(raw)
        assert stats["status"] == "active"

    def test_reset_clears_counters(self, browser):
        browser.rate_limit_record()
        browser.rate_limit_reset()
        raw = browser.rate_limit_stats()
        stats = json.loads(raw)
        assert stats["total_requests"] == 0
        assert stats["total_throttled"] == 0

    def test_presets_map(self, browser):
        raw = browser.rate_limit_presets()
        presets = json.loads(raw)
        assert "conservative" in presets
        assert "moderate" in presets
        assert "aggressive" in presets
        assert "unlimited" in presets

    def test_set_with_json_config(self, browser):
        cfg = json.dumps({
            "max_requests_per_second": 10,
            "max_requests_per_minute": 100,
            "max_requests_per_hour": 5000,
            "burst_size": 20,
            "cooldown_ms": 50,
        })
        raw = browser.rate_limit_set(cfg)
        stats = json.loads(raw)
        assert stats["status"] == "active"


class TestRetryQueue:
    def test_enqueue_returns_id(self, browser):
        item_id = browser.retry_enqueue("https://example.com", "navigate")
        assert isinstance(item_id, str)
        assert item_id.startswith("retry-")

    def test_next_returns_item(self, browser):
        raw = browser.retry_next()
        assert raw is not None
        item = json.loads(raw)
        assert "id" in item
        assert "url" in item

    def test_success_moves_to_completed(self, browser):
        item_id = browser.retry_enqueue("https://a.com", "click")
        browser.retry_success(item_id)
        raw = browser.retry_stats()
        stats = json.loads(raw)
        assert stats["completed_success"] >= 1

    def test_fail_increments_retries(self, browser):
        item_id = browser.retry_enqueue("https://b.com", "extract")
        browser.retry_fail(item_id, "timeout")
        raw = browser.retry_stats()
        stats = json.loads(raw)
        assert stats["total_retries"] >= 1

    def test_stats_valid_json(self, browser):
        raw = browser.retry_stats()
        stats = json.loads(raw)
        assert isinstance(stats["pending"], int)
        assert isinstance(stats["retrying"], int)
        assert isinstance(stats["completed_success"], int)
        assert isinstance(stats["completed_failed"], int)

    def test_clear_removes_completed(self, browser):
        item_id = browser.retry_enqueue("https://c.com", "submit")
        browser.retry_success(item_id)
        cleared = browser.retry_clear()
        assert cleared >= 1

    def test_save_and_load_roundtrip(self, browser):
        browser.retry_enqueue("https://d.com", "navigate", "test-payload")
        path = os.path.join(tempfile.gettempdir(), "onecrawl_retry_pyo3_test.json")
        browser.retry_save(path)
        browser.retry_load(path)
        raw = browser.retry_stats()
        stats = json.loads(raw)
        assert stats["pending"] >= 1
        if os.path.exists(path):
            os.remove(path)

    def test_enqueue_with_payload(self, browser):
        item_id = browser.retry_enqueue("https://e.com", "submit", "my-data")
        assert isinstance(item_id, str)
