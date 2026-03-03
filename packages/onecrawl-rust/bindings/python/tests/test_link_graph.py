"""Tests for the Link Graph module."""

import json
import os
import tempfile
import pytest
from onecrawl import Browser

EDGES = [
    {"source": "https://a.com/", "target": "https://a.com/about", "anchor_text": "About", "is_internal": True},
    {"source": "https://a.com/", "target": "https://a.com/blog", "anchor_text": "Blog", "is_internal": True},
    {"source": "https://a.com/", "target": "https://external.com", "anchor_text": "Ext", "is_internal": False},
    {"source": "https://a.com/about", "target": "https://a.com/", "anchor_text": "Home", "is_internal": True},
    {"source": "https://a.com/blog", "target": "https://a.com/", "anchor_text": "Home", "is_internal": True},
]


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestLinkGraph:
    def test_graph_build_creates_nodes_edges(self, browser):
        raw = browser.graph_build(json.dumps(EDGES))
        graph = json.loads(raw)
        assert isinstance(graph["nodes"], list)
        assert len(graph["nodes"]) >= 3
        assert len(graph["edges"]) == 5
        assert graph["total_internal"] == 4
        assert graph["total_external"] == 1

    def test_graph_analyze_stats(self, browser):
        graph = browser.graph_build(json.dumps(EDGES))
        raw = browser.graph_analyze(graph)
        stats = json.loads(raw)
        assert stats["total_edges"] == 5
        assert stats["total_nodes"] >= 3
        assert stats["avg_outbound"] > 0
        assert len(stats["max_inbound_url"]) > 0

    def test_graph_find_orphans(self, browser):
        graph = browser.graph_build(json.dumps(EDGES))
        orphans = json.loads(browser.graph_find_orphans(graph))
        assert isinstance(orphans, list)
        assert "https://a.com/about" not in orphans

    def test_graph_find_hubs(self, browser):
        graph = browser.graph_build(json.dumps(EDGES))
        hubs = json.loads(browser.graph_find_hubs(graph, 2))
        assert isinstance(hubs, list)
        assert any(h["url"] == "https://a.com/" for h in hubs)

    def test_graph_export_writes_file(self, browser):
        graph = browser.graph_build(json.dumps(EDGES))
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            tmp = f.name
        try:
            browser.graph_export(graph, tmp)
            data = json.loads(open(tmp).read())
            assert len(data["nodes"]) >= 3
        finally:
            os.unlink(tmp)

    def test_graph_from_crawl_results(self, browser):
        results = [
            {"url": "https://a.com/", "status": "success", "title": "Home", "depth": 0, "links_found": 2, "content": None, "error": None, "duration_ms": 10, "timestamp": 0},
            {"url": "https://a.com/about", "status": "success", "title": "About", "depth": 1, "links_found": 1, "content": None, "error": None, "duration_ms": 10, "timestamp": 0},
            {"url": "https://a.com/blog", "status": "success", "title": "Blog", "depth": 1, "links_found": 0, "content": None, "error": None, "duration_ms": 10, "timestamp": 0},
        ]
        raw = browser.graph_from_crawl_results(json.dumps(results))
        graph = json.loads(raw)
        assert len(graph["nodes"]) >= 2
        assert len(graph["edges"]) >= 2

    def test_graph_build_empty(self, browser):
        raw = browser.graph_build(json.dumps([]))
        graph = json.loads(raw)
        assert graph["nodes"] == []
        assert graph["edges"] == []
        assert graph["total_internal"] == 0
        assert graph["total_external"] == 0

    def test_graph_analyze_empty(self, browser):
        graph = browser.graph_build(json.dumps([]))
        stats = json.loads(browser.graph_analyze(graph))
        assert stats["total_nodes"] == 0
        assert stats["total_edges"] == 0
        assert stats["avg_inbound"] == 0
        assert stats["avg_outbound"] == 0
