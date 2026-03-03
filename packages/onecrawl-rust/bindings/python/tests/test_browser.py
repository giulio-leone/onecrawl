"""Browser E2E tests for the OneCrawl Python bindings (PyO3 + chromiumoxide)."""
import pytest


@pytest.fixture(scope="module")
def browser():
    from onecrawl import Browser
    b = Browser.launch(headless=True)
    yield b
    b.close()


def test_goto_and_title(browser):
    browser.goto("https://example.com")
    assert browser.get_title() == "Example Domain"


def test_get_url(browser):
    assert "example.com" in browser.get_url()


def test_content_html(browser):
    html = browser.content()
    assert "Example Domain" in html
    assert "<h1>" in html


def test_screenshot_png(browser):
    png = browser.screenshot()
    assert len(png) > 1000
    assert png[:4] == b'\x89PNG'


def test_screenshot_full(browser):
    png = browser.screenshot_full()
    assert len(png) > 1000


def test_evaluate_js(browser):
    result = browser.evaluate("document.title")
    assert "Example Domain" in result


def test_get_text(browser):
    text = browser.get_text("h1")
    assert text == "Example Domain"


def test_get_attribute(browser):
    href = browser.get_attribute("a", "href")
    assert "iana.org" in href


def test_click(browser):
    browser.goto("https://example.com")
    browser.click("h1")  # should not raise


def test_reload(browser):
    browser.goto("https://example.com")
    browser.reload()
    browser.wait(500)
    assert browser.get_title() == "Example Domain"


def test_wait_for_selector(browser):
    browser.wait_for_selector("h1", timeout_ms=5000)


def test_inject_stealth(browser):
    platform, hw, mem = browser.inject_stealth()
    assert platform
    assert hw > 0
    assert mem > 0
    result = browser.evaluate("String(navigator.webdriver)")
    assert "false" in result


def test_set_content(browser):
    browser.set_content('<html><body><h1 id="test">Custom</h1></body></html>')
    text = browser.get_text("#test")
    assert text == "Custom"


def test_new_page(browser):
    browser.new_page("https://example.com")
    browser.wait(500)
    assert browser.get_title() == "Example Domain"
