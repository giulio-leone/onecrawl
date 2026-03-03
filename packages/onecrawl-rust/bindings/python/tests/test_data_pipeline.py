"""Tests for the Data Pipeline module."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser


ITEMS = [
    {"name": "Alice", "age": "30", "city": "Rome"},
    {"name": "Bob", "age": "25", "city": "Milan"},
    {"name": "Charlie", "age": "30", "city": "Rome"},
    {"name": "Alice", "age": "35", "city": "Naples"},
    {"name": "Diana", "age": "28", "city": "Turin"},
]


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestDataPipeline:
    def test_filter_step(self, browser):
        pipeline = json.dumps({
            "name": "filter-test",
            "steps": [{"Filter": {"field": "city", "operator": "eq", "value": "Rome"}}],
        })
        raw = browser.pipeline_execute(pipeline, json.dumps(ITEMS))
        result = json.loads(raw)
        assert result["input_count"] == 5
        assert result["output_count"] == 2
        assert result["filtered_count"] == 3
        assert all(i["city"] == "Rome" for i in result["items"])

    def test_transform_step(self, browser):
        pipeline = json.dumps({
            "name": "transform-test",
            "steps": [{"Transform": {"field": "name", "transform": "uppercase"}}],
        })
        raw = browser.pipeline_execute(pipeline, json.dumps(ITEMS))
        result = json.loads(raw)
        assert result["output_count"] == 5
        assert result["items"][0]["name"] == "ALICE"
        assert result["items"][1]["name"] == "BOB"

    def test_deduplicate_step(self, browser):
        pipeline = json.dumps({
            "name": "dedup-test",
            "steps": [{"Deduplicate": {"field": "name"}}],
        })
        raw = browser.pipeline_execute(pipeline, json.dumps(ITEMS))
        result = json.loads(raw)
        assert result["output_count"] == 4
        assert result["deduplicated_count"] == 1

    def test_sort_step(self, browser):
        pipeline = json.dumps({
            "name": "sort-test",
            "steps": [{"Sort": {"field": "age", "descending": True}}],
        })
        raw = browser.pipeline_execute(pipeline, json.dumps(ITEMS))
        result = json.loads(raw)
        assert result["items"][0]["age"] == "35"
        assert result["items"][-1]["age"] == "25"

    def test_validate_pipeline_errors(self, browser):
        pipeline = json.dumps({
            "name": "",
            "steps": [{"Filter": {"field": "", "operator": "invalid", "value": ""}}],
        })
        raw = browser.pipeline_validate(pipeline)
        errors = json.loads(raw)
        assert len(errors) >= 2
        assert any("pipeline name is empty" in e for e in errors)

    def test_save_and_load_roundtrip(self, browser):
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp = f.name
        try:
            pipeline = json.dumps({
                "name": "roundtrip",
                "steps": [{"Limit": {"count": 10}}],
            })
            browser.pipeline_save(pipeline, tmp)
            loaded = browser.pipeline_load(tmp)
            parsed = json.loads(loaded)
            assert parsed["name"] == "roundtrip"
            assert len(parsed["steps"]) == 1
        finally:
            os.unlink(tmp)

    def test_export_csv(self, browser):
        pipeline = json.dumps({
            "name": "export-test",
            "steps": [{"Limit": {"count": 2}}],
        })
        raw = browser.pipeline_execute(pipeline, json.dumps(ITEMS))
        with tempfile.NamedTemporaryFile(suffix=".csv", delete=False) as f:
            tmp = f.name
        try:
            count = browser.pipeline_export(raw, tmp, "csv")
            assert count == 2
            csv = open(tmp).read()
            assert "name" in csv
            assert "Alice" in csv
        finally:
            os.unlink(tmp)

    def test_multi_step_pipeline(self, browser):
        pipeline = json.dumps({
            "name": "multi-step",
            "steps": [
                {"Filter": {"field": "city", "operator": "neq", "value": "Turin"}},
                {"Deduplicate": {"field": "name"}},
                {"Transform": {"field": "name", "transform": "lowercase"}},
                {"Sort": {"field": "age", "descending": False}},
                {"Limit": {"count": 2}},
            ],
        })
        raw = browser.pipeline_execute(pipeline, json.dumps(ITEMS))
        result = json.loads(raw)
        assert result["input_count"] == 5
        assert result["output_count"] <= 2
        assert all(i["name"] == i["name"].lower() for i in result["items"])
