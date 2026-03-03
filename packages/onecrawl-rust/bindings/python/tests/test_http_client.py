"""Tests for the HTTP Client module (browser fetch)."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestHttpClient:
    def test_http_get(self, browser):
        browser.goto("https://httpbin.org/get")
        resp = json.loads(browser.http_get("https://httpbin.org/get"))
        assert resp["status"] == 200
        assert len(resp["body"]) > 0
        assert resp["redirected"] is False
        assert resp["duration_ms"] >= 0

    def test_http_post(self, browser):
        browser.goto("https://httpbin.org/post")
        resp = json.loads(
            browser.http_post("https://httpbin.org/post", '{"key":"value"}', "application/json")
        )
        assert resp["status"] == 200
        body = json.loads(resp["body"])
        assert body["json"]["key"] == "value"

    def test_http_head(self, browser):
        browser.goto("https://httpbin.org/get")
        resp = json.loads(browser.http_head("https://httpbin.org/get"))
        assert resp["status"] == 200
        assert resp["body"] == ""

    def test_http_fetch_custom(self, browser):
        browser.goto("https://httpbin.org/get")
        req = json.dumps({
            "url": "https://httpbin.org/headers",
            "method": "GET",
            "headers": {"X-Custom": "test-value"},
            "body": None,
            "timeout_ms": 10000,
        })
        resp = json.loads(browser.http_fetch(req))
        assert resp["status"] == 200
        body = json.loads(resp["body"])
        assert body["headers"]["X-Custom"] == "test-value"

    def test_http_fetch_json(self, browser):
        browser.goto("https://httpbin.org/get")
        data = json.loads(browser.http_fetch_json("https://httpbin.org/get"))
        assert "url" in data
        assert "headers" in data

    def test_http_get_with_headers(self, browser):
        browser.goto("https://httpbin.org/get")
        headers = json.dumps({"Accept-Language": "it-IT"})
        resp = json.loads(browser.http_get("https://httpbin.org/headers", headers))
        assert resp["status"] == 200
        body = json.loads(resp["body"])
        assert body["headers"]["Accept-Language"] == "it-IT"

    def test_http_get_url_field(self, browser):
        browser.goto("https://httpbin.org/get")
        resp = json.loads(browser.http_get("https://httpbin.org/get"))
        assert "httpbin.org" in resp["url"]

    def test_http_get_status_text(self, browser):
        browser.goto("https://httpbin.org/get")
        resp = json.loads(browser.http_get("https://httpbin.org/get"))
        assert isinstance(resp["status_text"], str)
