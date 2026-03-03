"""Tests for Task Scheduler and Session Pool."""

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


class TestScheduler:
    def test_add_task_returns_id(self, browser):
        schedule = json.dumps({"interval_ms": 1000, "delay_ms": 0, "max_runs": None})
        task_id = browser.scheduler_add_task("nav-test", "navigate", "{}", schedule)
        assert task_id.startswith("task-")

    def test_list_tasks_non_empty(self, browser):
        raw = browser.scheduler_list_tasks()
        tasks = json.loads(raw)
        assert isinstance(tasks, list)
        assert len(tasks) >= 1

    def test_get_stats(self, browser):
        raw = browser.scheduler_get_stats()
        stats = json.loads(raw)
        assert "active" in stats
        assert "total" in stats

    def test_pause_task(self, browser):
        schedule = json.dumps({"interval_ms": 5000, "delay_ms": 0, "max_runs": None})
        task_id = browser.scheduler_add_task("pause-me", "extract", "{}", schedule)
        assert browser.scheduler_pause_task(task_id) is True

    def test_resume_task(self, browser):
        schedule = json.dumps({"interval_ms": 5000, "delay_ms": 0, "max_runs": None})
        task_id = browser.scheduler_add_task("resume-me", "crawl", "{}", schedule)
        browser.scheduler_pause_task(task_id)
        assert browser.scheduler_resume_task(task_id) is True

    def test_remove_task(self, browser):
        schedule = json.dumps({"interval_ms": 0, "delay_ms": 0, "max_runs": 1})
        task_id = browser.scheduler_add_task("remove-me", "screenshot", "{}", schedule)
        assert browser.scheduler_remove_task(task_id) is True
        assert browser.scheduler_remove_task(task_id) is False

    def test_get_due_tasks(self, browser):
        raw = browser.scheduler_get_due_tasks()
        due = json.loads(raw)
        assert isinstance(due, list)

    def test_save_and_load(self, browser):
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            browser.scheduler_save(path)
            assert os.path.exists(path)
            browser.scheduler_load(path)
        finally:
            if os.path.exists(path):
                os.unlink(path)


class TestSessionPool:
    def test_add_session_returns_id(self, browser):
        sid = browser.pool_add_session("worker-1")
        assert sid.startswith("sess-")

    def test_get_next_returns_session(self, browser):
        browser.pool_add_session("worker-2")
        raw = browser.pool_get_next()
        assert raw is not None
        session = json.loads(raw)
        assert "id" in session
        assert "status" in session

    def test_get_stats(self, browser):
        raw = browser.pool_get_stats()
        stats = json.loads(raw)
        assert "total" in stats
        assert "idle" in stats
        assert "busy" in stats

    def test_mark_busy(self, browser):
        sid = browser.pool_add_session("busy-test")
        browser.pool_mark_busy(sid)
        raw = browser.pool_get_stats()
        stats = json.loads(raw)
        assert stats["busy"] >= 1

    def test_mark_idle(self, browser):
        sid = browser.pool_add_session("idle-test")
        browser.pool_mark_busy(sid)
        browser.pool_mark_idle(sid)
        raw = browser.pool_get_stats()
        stats = json.loads(raw)
        assert stats["idle"] >= 1

    def test_close_session(self, browser):
        sid = browser.pool_add_session("close-test")
        browser.pool_close_session(sid)
        raw = browser.pool_get_stats()
        stats = json.loads(raw)
        assert stats["closed"] >= 1

    def test_cleanup_idle(self, browser):
        count = browser.pool_cleanup_idle()
        assert isinstance(count, int)

    def test_save_and_load(self, browser):
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            browser.pool_save(path)
            assert os.path.exists(path)
            browser.pool_load(path)
        finally:
            if os.path.exists(path):
                os.unlink(path)
