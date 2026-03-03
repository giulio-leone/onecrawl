"""Tests for Anti-Bot Bypass and Adaptive Element Tracker."""

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


class TestAntibot:
    def test_inject_stealth_full_returns_patches(self, browser):
        browser.goto("data:text/html,<h1>Antibot</h1>")
        raw = browser.inject_stealth_full()
        patches = json.loads(raw)
        assert isinstance(patches, list)
        assert len(patches) > 0
        assert "webdriver" in patches
        assert "chrome_runtime" in patches

    def test_bot_detection_test_returns_score(self, browser):
        raw = browser.bot_detection_test()
        result = json.loads(raw)
        assert isinstance(result, dict)
        assert "score" in result
        assert 0 <= result["score"] <= 100

    def test_bot_detection_test_fields(self, browser):
        result = json.loads(browser.bot_detection_test())
        assert "chrome" in result
        assert "plugins_length" in result
        assert "screen" in result
        assert "visibility_state" in result
        assert "hardware_concurrency" in result

    def test_stealth_profiles_returns_three(self, browser):
        raw = browser.stealth_profiles()
        profiles = json.loads(raw)
        assert isinstance(profiles, list)
        assert len(profiles) == 3
        names = [p["name"] for p in profiles]
        assert "basic" in names
        assert "standard" in names
        assert "aggressive" in names

    def test_stealth_profiles_aggressive_patches(self, browser):
        profiles = json.loads(browser.stealth_profiles())
        aggressive = next(p for p in profiles if p["name"] == "aggressive")
        assert len(aggressive["patches"]) > 10
        assert "canvas" in aggressive["patches"]
        assert "audio" in aggressive["patches"]

    def test_inject_stealth_full_idempotent(self, browser):
        p1 = json.loads(browser.inject_stealth_full())
        p2 = json.loads(browser.inject_stealth_full())
        assert p1 == p2


class TestAdaptive:
    def test_fingerprint_element(self, browser):
        browser.goto('data:text/html,<div id="target" class="box big">Hello World</div>')
        raw = browser.fingerprint_element("#target")
        fp = json.loads(raw)
        assert fp["tag"] == "div"
        assert "box" in fp["classes"]
        assert "big" in fp["classes"]
        assert "Hello World" in fp["text_preview"]

    def test_fingerprint_element_missing(self, browser):
        with pytest.raises(Exception):
            browser.fingerprint_element("#nonexistent")

    def test_relocate_element_exact_match(self, browser):
        browser.goto('data:text/html,<div id="target" class="box">Content</div>')
        fp_json = browser.fingerprint_element("#target")
        matches = json.loads(browser.relocate_element(fp_json))
        assert len(matches) > 0
        assert matches[0]["score"] == 100
        assert matches[0]["match_type"] == "exact"

    def test_track_elements(self, browser):
        browser.goto('data:text/html,<h1>Title</h1><p id="para">Text</p>')
        selectors = json.dumps(["h1", "#para"])
        fps = json.loads(browser.track_elements(selectors))
        assert len(fps) == 2
        assert fps[0]["tag"] == "h1"
        assert fps[1]["tag"] == "p"

    def test_relocate_all(self, browser):
        browser.goto('data:text/html,<h1>Title</h1><p id="para">Text</p>')
        fp_json = browser.track_elements(json.dumps(["h1", "#para"]))
        results = json.loads(browser.relocate_all(fp_json))
        assert len(results) == 2
        assert len(results[0][1]) > 0
        assert len(results[1][1]) > 0

    def test_save_load_fingerprints_roundtrip(self, browser):
        browser.goto('data:text/html,<div id="rt" class="test">Roundtrip</div>')
        fp_json = browser.fingerprint_element("#rt")
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp_path = f.name
        try:
            browser.save_fingerprints("[" + fp_json + "]", tmp_path)
            loaded = json.loads(browser.load_fingerprints(tmp_path))
            assert len(loaded) == 1
            assert loaded[0]["tag"] == "div"
            assert "test" in loaded[0]["classes"]
        finally:
            os.unlink(tmp_path)
