"""Tests for advanced browser operations: cookies, keyboard, input."""
import json
import pytest


def test_cookies_roundtrip():
    """Set, get, delete cookies."""
    from onecrawl import Browser

    browser = Browser.launch(headless=True)
    try:
        browser.goto("https://example.com")

        # Get cookies (might be empty initially)
        cookies_json = browser.get_cookies()
        cookies = json.loads(cookies_json)
        assert isinstance(cookies, list)

        # Set a cookie
        browser.set_cookie(json.dumps({
            "name": "py_test",
            "value": "hello456",
            "domain": "example.com",
            "path": "/",
        }))

        # Verify it exists
        cookies_json = browser.get_cookies()
        cookies = json.loads(cookies_json)
        found = [c for c in cookies if c["name"] == "py_test"]
        assert len(found) == 1
        assert found[0]["value"] == "hello456"

        # Delete it
        browser.delete_cookies("py_test", domain="example.com")
        cookies_json = browser.get_cookies()
        cookies = json.loads(cookies_json)
        found = [c for c in cookies if c["name"] == "py_test"]
        assert len(found) == 0

        # Clear all
        browser.clear_cookies()
        cookies_json = browser.get_cookies()
        cookies = json.loads(cookies_json)
        assert len(cookies) == 0
    finally:
        browser.close()


def test_keyboard_operations():
    """Press keys, shortcuts, fill."""
    from onecrawl import Browser

    browser = Browser.launch(headless=True)
    try:
        browser.set_content('<input id="kb" type="text" autofocus />')
        browser.click("#kb")
        browser.press_key("a")
        browser.keyboard_shortcut("Control+a")
        browser.key_down("Shift")
        browser.key_up("Shift")
    finally:
        browser.close()


def test_fill_input():
    """Fill clears and sets input value."""
    from onecrawl import Browser

    browser = Browser.launch(headless=True)
    try:
        browser.set_content('<input id="f" type="text" />')
        browser.fill("#f", "python_value")
        val = browser.evaluate("document.querySelector('#f').value")
        assert "python_value" in val
    finally:
        browser.close()


def test_bounding_box():
    """Get element dimensions."""
    from onecrawl import Browser

    browser = Browser.launch(headless=True)
    try:
        browser.set_content('<div id="box" style="width:200px;height:100px;">Box</div>')
        x, y, w, h = browser.bounding_box("#box")
        assert w > 0
        assert h > 0
    finally:
        browser.close()


def test_tap():
    """Tap element (touch simulation)."""
    from onecrawl import Browser

    browser = Browser.launch(headless=True)
    try:
        browser.set_content('<button id="tapme">Tap</button>')
        browser.tap("#tapme")
    finally:
        browser.close()


def test_drag_and_drop():
    """Drag-and-drop between elements."""
    from onecrawl import Browser

    browser = Browser.launch(headless=True)
    try:
        browser.set_content("""
            <div id="src" draggable="true" style="width:50px;height:50px;background:red;">S</div>
            <div id="tgt" style="width:100px;height:100px;background:blue;">T</div>
        """)
        browser.drag_and_drop("#src", "#tgt")
    finally:
        browser.close()
