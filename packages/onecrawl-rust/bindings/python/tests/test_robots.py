"""Tests for the Robots.txt module."""

import json
import pytest
from onecrawl import Browser

ROBOTS_TXT = """
# Example robots.txt
User-agent: Googlebot
Allow: /public/
Disallow: /private/
Crawl-delay: 2

User-agent: *
Disallow: /admin/
Disallow: /tmp/

Sitemap: https://example.com/sitemap.xml
Sitemap: https://example.com/sitemap2.xml
"""


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestRobots:
    def test_robots_parse_rules_and_sitemaps(self, browser):
        raw = browser.robots_parse(ROBOTS_TXT)
        robots = json.loads(raw)
        assert isinstance(robots["rules"], list)
        assert len(robots["rules"]) >= 2
        assert robots["sitemaps"] == [
            "https://example.com/sitemap.xml",
            "https://example.com/sitemap2.xml",
        ]

    def test_robots_is_allowed_permits(self, browser):
        robots = browser.robots_parse(ROBOTS_TXT)
        assert browser.robots_is_allowed(robots, "Googlebot", "/public/page") is True
        assert browser.robots_is_allowed(robots, "Googlebot", "/private/secret") is False

    def test_robots_is_allowed_wildcard(self, browser):
        robots = browser.robots_parse(ROBOTS_TXT)
        assert browser.robots_is_allowed(robots, "RandomBot", "/admin/settings") is False
        assert browser.robots_is_allowed(robots, "RandomBot", "/about") is True

    def test_robots_crawl_delay_match(self, browser):
        robots = browser.robots_parse(ROBOTS_TXT)
        delay = browser.robots_crawl_delay(robots, "Googlebot")
        assert delay == 2.0

    def test_robots_crawl_delay_none(self, browser):
        robots = browser.robots_parse(ROBOTS_TXT)
        delay = browser.robots_crawl_delay(robots, "RandomBot")
        assert delay is None

    def test_robots_sitemaps(self, browser):
        robots = browser.robots_parse(ROBOTS_TXT)
        sitemaps = json.loads(browser.robots_sitemaps(robots))
        assert isinstance(sitemaps, list)
        assert len(sitemaps) == 2
        assert "sitemap.xml" in sitemaps[0]

    def test_robots_parse_empty(self, browser):
        raw = browser.robots_parse("")
        robots = json.loads(raw)
        assert robots["rules"] == []
        assert robots["sitemaps"] == []

    def test_robots_parse_comments_only(self, browser):
        raw = browser.robots_parse("# just a comment\n# another comment")
        robots = json.loads(raw)
        assert robots["rules"] == []
