"""Tests for TLS Fingerprint and Page Snapshot modules."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    b.goto("https://example.com")
    yield b
    b.close()


# ── TLS Fingerprint ──────────────────────────────────────────


def test_fingerprint_profiles_returns_six(browser):
    profiles = json.loads(browser.fingerprint_profiles())
    assert isinstance(profiles, list)
    assert len(profiles) == 6
    names = [p["name"] for p in profiles]
    assert "chrome-win" in names
    assert "safari-mac" in names
    assert "edge-win" in names


def test_apply_fingerprint_chrome_win(browser):
    browser.goto("data:text/html,<h1>FP</h1>")
    result = json.loads(browser.apply_fingerprint("chrome-win"))
    assert isinstance(result, list)
    assert len(result) > 0
    assert "userAgent" in result
    assert "platform" in result


def test_apply_fingerprint_unknown_raises(browser):
    with pytest.raises(Exception, match="unknown fingerprint"):
        browser.apply_fingerprint("nonexistent")


def test_apply_random_fingerprint(browser):
    browser.goto("data:text/html,<h1>Random</h1>")
    fp = json.loads(browser.apply_random_fingerprint())
    assert fp["name"] == "random"
    assert len(fp["user_agent"]) > 0
    assert fp["screen_width"] > 0


def test_detect_fingerprint(browser):
    browser.goto("data:text/html,<h1>Detect</h1>")
    fp = json.loads(browser.detect_fingerprint())
    assert fp["name"] == "detected"
    assert isinstance(fp["user_agent"], str)
    assert isinstance(fp["hardware_concurrency"], int)
    assert fp["screen_width"] > 0


def test_apply_custom_fingerprint(browser):
    browser.goto("data:text/html,<h1>Custom</h1>")
    profiles = json.loads(browser.fingerprint_profiles())
    custom = profiles[0]
    custom["name"] = "custom-test"
    result = json.loads(browser.apply_custom_fingerprint(json.dumps(custom)))
    assert "userAgent" in result


def test_apply_custom_fingerprint_invalid_json(browser):
    with pytest.raises(Exception, match="invalid fingerprint"):
        browser.apply_custom_fingerprint("{bad")


def test_apply_fingerprint_changes_platform(browser):
    browser.goto("data:text/html,<h1>Platform</h1>")
    browser.apply_fingerprint("firefox-mac")
    platform = json.loads(browser.evaluate("navigator.platform"))
    assert platform == "MacIntel"


# ── Page Snapshot ─────────────────────────────────────────────


def test_take_snapshot(browser):
    browser.goto('data:text/html,<html><head><title>Snap</title></head><body><p>Hello</p><a href="https://example.com">Link</a></body></html>')
    snap = json.loads(browser.take_snapshot())
    assert snap["title"] == "Snap"
    assert "Hello" in snap["text"]
    assert len(snap["links"]) > 0
    assert snap["element_count"] > 0
    assert snap["word_count"] > 0


def test_compare_snapshots_identical(browser):
    browser.goto("data:text/html,<p>Same</p>")
    snap = browser.take_snapshot()
    diff = json.loads(browser.compare_snapshots(snap, snap))
    assert diff["title_changed"] is False
    assert diff["html_changed"] is False
    assert diff["text_changed"] is False
    assert diff["similarity"] == 1.0
    assert len(diff["links_added"]) == 0


def test_compare_snapshots_text_changed(browser):
    browser.goto("data:text/html,<p>Version 1</p>")
    snap1 = browser.take_snapshot()
    browser.goto("data:text/html,<p>Version 2</p>")
    snap2 = browser.take_snapshot()
    diff = json.loads(browser.compare_snapshots(snap1, snap2))
    assert diff["text_changed"] is True
    assert diff["similarity"] < 1.0


def test_save_load_snapshot_roundtrip(browser):
    browser.goto("data:text/html,<p>Save me</p>")
    snap_json = browser.take_snapshot()
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
        tmp_path = f.name
    try:
        browser.save_snapshot(snap_json, tmp_path)
        loaded = json.loads(browser.load_snapshot(tmp_path))
        original = json.loads(snap_json)
        assert loaded["title"] == original["title"]
        assert loaded["text"] == original["text"]
    finally:
        os.unlink(tmp_path)


def test_load_snapshot_missing_file(browser):
    with pytest.raises(Exception):
        browser.load_snapshot("/tmp/nonexistent-snap.json")


def test_compare_snapshots_detects_added_links(browser):
    browser.goto('data:text/html,<a href="https://a.com">A</a>')
    snap1 = browser.take_snapshot()
    browser.goto('data:text/html,<a href="https://a.com">A</a><a href="https://b.com">B</a>')
    snap2 = browser.take_snapshot()
    diff = json.loads(browser.compare_snapshots(snap1, snap2))
    assert len(diff["links_added"]) > 0


def test_compare_snapshots_element_count_delta(browser):
    browser.goto("data:text/html,<div><p>One</p></div>")
    snap1 = browser.take_snapshot()
    browser.goto("data:text/html,<div><p>One</p><p>Two</p><p>Three</p></div>")
    snap2 = browser.take_snapshot()
    diff = json.loads(browser.compare_snapshots(snap1, snap2))
    assert diff["element_count_delta"] > 0


def test_take_snapshot_has_meta(browser):
    browser.goto('data:text/html,<html><head><meta name="description" content="test desc"></head><body>Hi</body></html>')
    snap = json.loads(browser.take_snapshot())
    assert isinstance(snap["meta"], dict)
    assert snap["meta"]["description"] == "test desc"


def test_compare_snapshots_invalid_json(browser):
    with pytest.raises(Exception, match="invalid"):
        browser.compare_snapshots("{bad", "{}")
