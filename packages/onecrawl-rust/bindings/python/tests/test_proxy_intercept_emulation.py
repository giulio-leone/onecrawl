"""Tests for proxy pool, request interception, and advanced emulation."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    b.goto("https://example.com")
    yield b
    b.close()


# ── Proxy Pool (static methods, no page needed) ─────────────────


def test_create_proxy_pool():
    config = json.dumps(
        {
            "proxies": [
                {"server": "http://proxy1:8080", "username": None, "password": None, "bypass": None},
                {"server": "http://proxy2:8080", "username": "user", "password": "pass", "bypass": "localhost"},
            ],
            "strategy": "RoundRobin",
            "current_index": 0,
        }
    )
    result = Browser.create_proxy_pool(config)
    pool = json.loads(result)
    assert len(pool["proxies"]) == 2
    assert pool["strategy"] == "RoundRobin"


def test_get_proxy_chrome_args():
    pool = json.dumps(
        {
            "proxies": [{"server": "http://proxy1:8080", "username": None, "password": None, "bypass": "localhost,127.0.0.1"}],
            "strategy": "Sticky",
            "current_index": 0,
        }
    )
    args = Browser.get_proxy_chrome_args(pool)
    assert any("--proxy-server=http://proxy1:8080" in a for a in args)
    assert any("--proxy-bypass-list=localhost,127.0.0.1" in a for a in args)


def test_get_proxy_chrome_args_empty():
    pool = json.dumps({"proxies": [], "strategy": "RoundRobin", "current_index": 0})
    args = Browser.get_proxy_chrome_args(pool)
    assert len(args) == 0


def test_next_proxy():
    pool = json.dumps(
        {
            "proxies": [
                {"server": "http://p1:80", "username": None, "password": None, "bypass": None},
                {"server": "http://p2:80", "username": None, "password": None, "bypass": None},
            ],
            "strategy": "RoundRobin",
            "current_index": 0,
        }
    )
    updated = json.loads(Browser.next_proxy(pool))
    assert updated["current_index"] == 1


def test_create_proxy_pool_invalid():
    with pytest.raises(Exception):
        Browser.create_proxy_pool("not json")


# ── Request Interception ────────────────────────────────────────


def test_set_intercept_rules(browser):
    rules = json.dumps([{"url_pattern": "*blocked*", "resource_type": None, "action": "Block"}])
    browser.set_intercept_rules(rules)


def test_get_intercepted_requests(browser):
    log = json.loads(browser.get_intercepted_requests())
    assert isinstance(log, list)


def test_clear_intercept_rules(browser):
    browser.clear_intercept_rules()
    log = json.loads(browser.get_intercepted_requests())
    assert len(log) == 0


def test_set_intercept_rules_invalid(browser):
    with pytest.raises(Exception):
        browser.set_intercept_rules("bad")


# ── Advanced Emulation ──────────────────────────────────────────


def test_set_device_orientation(browser):
    browser.set_device_orientation(45.0, 90.0, 0.0)


def test_override_permission(browser):
    browser.override_permission("geolocation", "granted")
    state = json.loads(
        browser.evaluate("navigator.permissions.query({ name: 'geolocation' }).then(r => r.state)")
    )
    assert state == "granted"


def test_set_battery_status(browser):
    browser.set_battery_status(0.75, False)
    level = json.loads(browser.evaluate("navigator.getBattery().then(b => b.level)"))
    assert level == 0.75


def test_set_connection_info(browser):
    browser.set_connection_info("4g", 10.0, 50)
    etype = json.loads(browser.evaluate("navigator.connection.effectiveType"))
    assert etype == "4g"


def test_set_hardware_concurrency(browser):
    browser.set_hardware_concurrency(16)
    cores = json.loads(browser.evaluate("navigator.hardwareConcurrency"))
    assert cores == 16


def test_set_device_memory(browser):
    browser.set_device_memory(32.0)
    mem = json.loads(browser.evaluate("navigator.deviceMemory"))
    assert mem == 32


def test_get_navigator_info(browser):
    info = json.loads(browser.get_navigator_info())
    assert "userAgent" in info
    assert "platform" in info
    assert "hardwareConcurrency" in info
    assert "deviceMemory" in info
