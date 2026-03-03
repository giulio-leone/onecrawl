"""Tests for Console interception, Dialog handling, Workers, and Web Storage."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestConsoleInterception:
    def test_start_console_capture(self, browser):
        browser.goto("data:text/html,<h1>Console</h1>")
        browser.start_console_capture()

    def test_drain_console_entries(self, browser):
        browser.evaluate('console.log("hello from test")')
        raw = browser.drain_console_entries()
        entries = json.loads(raw)
        assert isinstance(entries, list)

    def test_captures_warn_messages(self, browser):
        browser.evaluate('console.warn("warn msg")')
        raw = browser.drain_console_entries()
        entries = json.loads(raw)
        warn = [e for e in entries if e["level"] == "warn"]
        assert len(warn) > 0
        assert "warn msg" in warn[0]["text"]

    def test_clear_console(self, browser):
        browser.clear_console()
        raw = browser.drain_console_entries()
        entries = json.loads(raw)
        assert len(entries) == 0


class TestDialogHandling:
    def test_set_dialog_handler(self, browser):
        browser.goto("data:text/html,<h1>Dialog</h1>")
        browser.set_dialog_handler(True)

    def test_get_dialog_history(self, browser):
        raw = browser.get_dialog_history()
        events = json.loads(raw)
        assert isinstance(events, list)

    def test_records_alert(self, browser):
        browser.evaluate('alert("test alert")')
        raw = browser.get_dialog_history()
        events = json.loads(raw)
        alerts = [e for e in events if e["dialog_type"] == "alert"]
        assert len(alerts) > 0
        assert "test alert" in alerts[0]["message"]

    def test_clear_dialog_history(self, browser):
        browser.clear_dialog_history()
        raw = browser.get_dialog_history()
        events = json.loads(raw)
        assert len(events) == 0


class TestServiceWorkers:
    def test_get_service_workers(self, browser):
        browser.goto("data:text/html,<h1>Workers</h1>")
        raw = browser.get_service_workers()
        workers = json.loads(raw)
        assert isinstance(workers, list)

    def test_unregister_service_workers(self, browser):
        count = browser.unregister_service_workers()
        assert count == 0

    def test_get_worker_info(self, browser):
        raw = browser.get_worker_info()
        info = json.loads(raw)
        assert isinstance(info, dict)


class TestWebStorage:
    def test_set_and_get_local_storage(self, browser):
        browser.goto("data:text/html,<h1>Storage</h1>")
        browser.set_local_storage("testKey", "testValue")
        raw = browser.get_local_storage()
        data = json.loads(raw)
        assert data["testKey"] == "testValue"

    def test_clear_local_storage(self, browser):
        browser.clear_local_storage()
        raw = browser.get_local_storage()
        data = json.loads(raw)
        assert len(data) == 0

    def test_set_and_get_session_storage(self, browser):
        browser.set_session_storage("sessKey", "sessVal")
        raw = browser.get_session_storage()
        data = json.loads(raw)
        assert data["sessKey"] == "sessVal"

    def test_clear_session_storage(self, browser):
        browser.clear_session_storage()
        raw = browser.get_session_storage()
        data = json.loads(raw)
        assert len(data) == 0

    def test_get_indexeddb_databases(self, browser):
        raw = browser.get_indexeddb_databases()
        names = json.loads(raw)
        assert isinstance(names, list)

    def test_clear_site_data(self, browser):
        browser.clear_site_data()
