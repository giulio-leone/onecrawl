"""Tests for Shell and Domain Blocker modules."""

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


class TestShell:
    def test_shell_parse_basic(self, browser):
        raw = browser.shell_parse("goto https://example.com")
        cmd = json.loads(raw)
        assert cmd["command"] == "goto"
        assert cmd["args"] == ["https://example.com"]
        assert isinstance(cmd["raw"], str)
        assert isinstance(cmd["timestamp"], float)

    def test_shell_parse_empty(self, browser):
        raw = browser.shell_parse("")
        cmd = json.loads(raw)
        assert cmd["command"] == ""
        assert cmd["args"] == []

    def test_shell_parse_multiple_args(self, browser):
        raw = browser.shell_parse("type #input hello world")
        cmd = json.loads(raw)
        assert cmd["command"] == "type"
        assert cmd["args"] == ["#input", "hello", "world"]

    def test_shell_commands(self, browser):
        raw = browser.shell_commands()
        cmds = json.loads(raw)
        assert isinstance(cmds, list)
        assert len(cmds) > 10
        names = [c[0] for c in cmds]
        assert any("goto" in n for n in names)
        assert any("exit" in n for n in names)

    def test_shell_save_load_history(self, browser):
        history = {
            "commands": [
                {
                    "raw": "goto https://example.com",
                    "command": "goto",
                    "args": ["https://example.com"],
                    "timestamp": 1000.0,
                }
            ],
            "max_size": 100,
        }
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            browser.shell_save_history(json.dumps(history), path)
            assert os.path.exists(path)
            loaded = json.loads(browser.shell_load_history(path))
            assert len(loaded["commands"]) == 1
            assert loaded["commands"][0]["command"] == "goto"
            assert loaded["max_size"] == 100
        finally:
            os.unlink(path)


class TestDomainBlocker:
    def test_block_domains(self, browser):
        browser.goto("data:text/html,<h1>Block</h1>")
        count = browser.block_domains(json.dumps(["evil.com", "tracker.io"]))
        assert count >= 2

    def test_list_blocked(self, browser):
        raw = browser.list_blocked()
        domains = json.loads(raw)
        assert isinstance(domains, list)
        assert "evil.com" in domains
        assert "tracker.io" in domains

    def test_block_category(self, browser):
        count = browser.block_category("ads")
        assert count > 10

    def test_block_stats(self, browser):
        raw = browser.block_stats()
        stats = json.loads(raw)
        assert isinstance(stats["total_blocked"], int)
        assert isinstance(stats["domains"], list)

    def test_available_block_categories(self, browser):
        raw = browser.available_block_categories()
        cats = json.loads(raw)
        assert isinstance(cats, list)
        assert len(cats) >= 5
        names = [c[0] for c in cats]
        assert "ads" in names
        assert "trackers" in names

    def test_clear_blocks(self, browser):
        browser.clear_blocks()
        raw = browser.list_blocked()
        domains = json.loads(raw)
        assert len(domains) == 0
