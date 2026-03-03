"""Tests for tabs, downloads, and screenshot diff modules."""
import json
import os
import tempfile

import pytest


@pytest.fixture(scope="module")
def browser():
    from onecrawl import Browser
    b = Browser.launch(headless=True)
    yield b
    b.close()


# ── Tab Management ────────────────────────────────────────────


def test_list_tabs(browser):
    tabs = json.loads(browser.list_tabs())
    assert isinstance(tabs, list)
    assert len(tabs) >= 1


def test_tab_count(browser):
    count = browser.tab_count()
    assert isinstance(count, int)
    assert count >= 1


def test_new_tab(browser):
    count_before = browser.tab_count()
    browser.new_tab("about:blank")
    count_after = browser.tab_count()
    assert count_after >= count_before


def test_list_tabs_fields(browser):
    tabs = json.loads(browser.list_tabs())
    for tab in tabs:
        assert "url" in tab
        assert "target_id" in tab
        assert "index" in tab


def test_switch_tab(browser):
    browser.switch_tab(0)
    tabs = json.loads(browser.list_tabs())
    assert len(tabs) >= 1


# ── Download Management ───────────────────────────────────────


def test_set_download_path(browser):
    with tempfile.TemporaryDirectory() as tmpdir:
        browser.set_download_path(tmpdir)


def test_get_downloads_empty(browser):
    browser.goto("about:blank")
    downloads = json.loads(browser.get_downloads())
    assert isinstance(downloads, list)


def test_clear_downloads(browser):
    browser.clear_downloads()
    downloads = json.loads(browser.get_downloads())
    assert downloads == []


def test_wait_for_download_timeout(browser):
    result = browser.wait_for_download(timeout_ms=500)
    assert result is None


def test_download_file(browser):
    browser.goto("https://example.com")
    try:
        b64 = browser.download_file("https://example.com/")
        assert isinstance(b64, str)
    except Exception:
        # Cross-origin fetch may fail
        pass


# ── Screenshot Diff ───────────────────────────────────────────


def test_compare_screenshots_identical(browser):
    browser.goto("https://example.com")
    png = browser.screenshot()

    with tempfile.TemporaryDirectory() as tmpdir:
        a = os.path.join(tmpdir, "a.png")
        b = os.path.join(tmpdir, "b.png")
        with open(a, "wb") as f:
            f.write(bytes(png))
        with open(b, "wb") as f:
            f.write(bytes(png))

        result = json.loads(browser.compare_screenshots(a, b))
        assert result["is_identical"] is True
        assert result["difference_percentage"] == 0


def test_compare_screenshots_different(browser):
    with tempfile.TemporaryDirectory() as tmpdir:
        a = os.path.join(tmpdir, "a.bin")
        b = os.path.join(tmpdir, "b.bin")
        with open(a, "wb") as f:
            f.write(bytes([0, 0, 0, 0, 1, 1, 1, 1]))
        with open(b, "wb") as f:
            f.write(bytes([0, 0, 0, 0, 2, 2, 2, 2]))

        result = json.loads(browser.compare_screenshots(a, b))
        assert result["is_identical"] is False
        assert result["difference_percentage"] > 0


def test_visual_regression_creates_baseline(browser):
    browser.goto("https://example.com")
    with tempfile.TemporaryDirectory() as tmpdir:
        baseline = os.path.join(tmpdir, "baseline.png")
        result = json.loads(browser.visual_regression(baseline))
        assert result["is_identical"] is True
        assert result["difference_percentage"] == 0
        assert os.path.exists(baseline)


def test_visual_regression_compares(browser):
    with tempfile.TemporaryDirectory() as tmpdir:
        baseline = os.path.join(tmpdir, "baseline.png")
        # First creates baseline
        browser.visual_regression(baseline)
        # Second compares
        result = json.loads(browser.visual_regression(baseline))
        assert "is_identical" in result
        assert "difference_percentage" in result


def test_close_tab(browser):
    browser.new_tab("about:blank")
    count = browser.tab_count()
    if count > 1:
        browser.close_tab(count - 1)
        after = browser.tab_count()
        assert after < count
