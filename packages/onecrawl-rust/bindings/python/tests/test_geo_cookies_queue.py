"""Tests for geofencing, cookie jar, and request queue modules."""

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


class TestGeofencing:
    def test_list_geo_presets(self, browser):
        presets = browser.list_geo_presets()
        assert isinstance(presets, list)
        assert len(presets) >= 8
        assert "New York" in presets
        assert "London" in presets
        assert "Tokyo" in presets

    def test_get_geo_preset_valid(self, browser):
        raw = browser.get_geo_preset("New York")
        assert raw is not None
        profile = json.loads(raw)
        assert profile["name"] == "New York"
        assert profile["latitude"] == 40.7128
        assert profile["timezone"] == "America/New_York"

    def test_get_geo_preset_case_insensitive(self, browser):
        raw = browser.get_geo_preset("new york")
        assert raw is not None
        profile = json.loads(raw)
        assert profile["name"] == "New York"

    def test_get_geo_preset_unknown(self, browser):
        result = browser.get_geo_preset("Atlantis")
        assert result is None

    def test_apply_geo_profile(self, browser):
        browser.goto("data:text/html,<h1>Geo</h1>")
        preset = browser.get_geo_preset("Tokyo")
        browser.apply_geo_profile(preset)


class TestCookieJar:
    def test_export_cookies(self, browser):
        browser.goto("data:text/html,<h1>CookieJar</h1>")
        raw = browser.export_cookies()
        jar = json.loads(raw)
        assert "version" in jar
        assert isinstance(jar["cookies"], list)
        assert "exported_at" in jar

    def test_import_cookies(self, browser):
        jar = json.dumps({
            "cookies": [{
                "name": "test_ck", "value": "abc123", "domain": "localhost",
                "path": "/", "expires": 0.0, "http_only": False, "secure": False,
                "same_site": None,
            }],
            "domain": None, "exported_at": "0", "version": "1.0",
        })
        count = browser.import_cookies(jar)
        assert count == 1

    def test_clear_all_cookies(self, browser):
        browser.clear_all_cookies()

    def test_save_cookies_to_file(self, browser):
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            count = browser.save_cookies_to_file(path)
            assert isinstance(count, int)
            assert os.path.exists(path)
        finally:
            os.unlink(path)

    def test_load_cookies_from_file(self, browser):
        jar = {"cookies": [], "domain": None, "exported_at": "0", "version": "1.0"}
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False, mode="w") as f:
            json.dump(jar, f)
            path = f.name
        try:
            count = browser.load_cookies_from_file(path)
            assert count == 0
        finally:
            os.unlink(path)


class TestRequestQueue:
    def test_create_get_request(self, browser):
        raw = browser.create_get_request("r1", "https://example.com")
        req = json.loads(raw)
        assert req["id"] == "r1"
        assert req["method"] == "GET"
        assert req["url"] == "https://example.com"
        assert req["max_retries"] == 3

    def test_create_post_request(self, browser):
        raw = browser.create_post_request("r2", "https://example.com/api", '{"key":"val"}')
        req = json.loads(raw)
        assert req["id"] == "r2"
        assert req["method"] == "POST"
        assert "Content-Type" in req["headers"]
        assert req["body"] == '{"key":"val"}'

    def test_execute_request(self, browser):
        browser.goto("data:text/html,<h1>Queue</h1>")
        req = browser.create_get_request("test-req", "data:text/html,hello")
        raw = browser.execute_request(req)
        result = json.loads(raw)
        assert result["id"] == "test-req"
        assert isinstance(result["attempts"], int)
        assert isinstance(result["duration_ms"], (int, float))

    def test_execute_batch(self, browser):
        reqs = json.dumps([
            json.loads(browser.create_get_request("b1", "data:text/html,one")),
            json.loads(browser.create_get_request("b2", "data:text/html,two")),
        ])
        raw = browser.execute_batch(reqs)
        results = json.loads(raw)
        assert isinstance(results, list)
        assert len(results) == 2

    def test_execute_batch_with_config(self, browser):
        reqs = json.dumps([
            json.loads(browser.create_get_request("c1", "data:text/html,cfg")),
        ])
        config = json.dumps({
            "concurrency": 1, "delay_between_ms": 0,
            "default_timeout_ms": 5000, "default_max_retries": 1,
            "default_retry_delay_ms": 100,
        })
        raw = browser.execute_batch(reqs, config)
        results = json.loads(raw)
        assert isinstance(results, list)
