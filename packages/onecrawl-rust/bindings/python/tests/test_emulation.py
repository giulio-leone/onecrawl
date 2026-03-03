"""Tests for Browser emulation methods (viewport, device, user agent, color scheme)."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    b.goto("https://example.com")
    yield b
    b.close()


def test_set_viewport(browser):
    browser.set_viewport(1920, 1080, device_scale_factor=2.0)
    w = json.loads(browser.evaluate("window.innerWidth"))
    h = json.loads(browser.evaluate("window.innerHeight"))
    assert w == 1920
    assert h == 1080


def test_set_device_iphone(browser):
    browser.set_device("iphone14")
    w = json.loads(browser.evaluate("window.innerWidth"))
    dpr = json.loads(browser.evaluate("window.devicePixelRatio"))
    assert w == 390
    assert dpr == 3


def test_set_device_desktop(browser):
    browser.set_device("desktop")
    w = json.loads(browser.evaluate("window.innerWidth"))
    assert w == 1280


def test_clear_viewport(browser):
    browser.set_viewport(400, 300)
    browser.clear_viewport()
    w = json.loads(browser.evaluate("window.innerWidth"))
    assert w > 0


def test_set_user_agent(browser):
    browser.set_user_agent("OneCrawl/1.0 PyBot")
    browser.goto("https://example.com")
    ua = json.loads(browser.evaluate("navigator.userAgent"))
    assert ua == "OneCrawl/1.0 PyBot"


def test_set_geolocation(browser):
    browser.set_geolocation(48.8566, 2.3522)


def test_set_color_scheme_dark(browser):
    browser.set_color_scheme("dark")
    is_dark = json.loads(browser.evaluate('window.matchMedia("(prefers-color-scheme: dark)").matches'))
    assert is_dark is True


def test_set_color_scheme_light(browser):
    browser.set_color_scheme("light")
    is_light = json.loads(browser.evaluate('window.matchMedia("(prefers-color-scheme: light)").matches'))
    assert is_light is True


def test_set_device_unknown_raises(browser):
    with pytest.raises(Exception, match="Unknown device"):
        browser.set_device("nonexistent")
