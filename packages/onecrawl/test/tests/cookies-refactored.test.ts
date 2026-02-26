/**
 * Cookies Cross-Platform Tests
 * Tests the refactored cookies module that works across platforms.
 */

import { describe, it, expect } from "vitest";
import {
  MemoryCookieStore,
  exportCookiesTxt,
  cookiesToHeader,
  parseSetCookie,
} from "../../src/auth/cookies.js";

describe("Cookies (Cross-Platform)", () => {
  describe("MemoryCookieStore", () => {
    it("should store and retrieve cookies", () => {
      const store = new MemoryCookieStore();
      store.setCookies([
        { name: "session", value: "abc123", domain: "example.com", path: "/" },
      ]);
      const cookies = store.getCookies("example.com");
      expect(cookies).toHaveLength(1);
      expect(cookies[0]!.name).toBe("session");
    });

    it("should match parent domains", () => {
      const store = new MemoryCookieStore();
      store.setCookies([
        { name: "global", value: "xyz", domain: "example.com", path: "/" },
      ]);
      const cookies = store.getCookies("www.example.com");
      expect(cookies).toHaveLength(1);
    });

    it("should skip expired cookies", () => {
      const store = new MemoryCookieStore();
      store.setCookies([
        {
          name: "expired",
          value: "old",
          domain: "example.com",
          path: "/",
          expires: 1,
        },
      ]);
      const cookies = store.getCookies("example.com");
      expect(cookies).toHaveLength(0);
    });

    it("should clear cookies by domain", () => {
      const store = new MemoryCookieStore();
      store.setCookies([
        { name: "a", value: "1", domain: "one.com", path: "/" },
        { name: "b", value: "2", domain: "two.com", path: "/" },
      ]);
      store.clearCookies("one.com");
      expect(store.getCookies("one.com")).toHaveLength(0);
      expect(store.getCookies("two.com")).toHaveLength(1);
    });

    it("should clear all cookies when no domain specified", () => {
      const store = new MemoryCookieStore();
      store.setCookies([
        { name: "a", value: "1", domain: "one.com", path: "/" },
        { name: "b", value: "2", domain: "two.com", path: "/" },
      ]);
      store.clearCookies();
      expect(store.getCookies("one.com")).toHaveLength(0);
      expect(store.getCookies("two.com")).toHaveLength(0);
    });

    it("should replace cookie with same name", () => {
      const store = new MemoryCookieStore();
      store.setCookies([
        { name: "token", value: "old", domain: "example.com", path: "/" },
      ]);
      store.setCookies([
        { name: "token", value: "new", domain: "example.com", path: "/" },
      ]);
      const cookies = store.getCookies("example.com");
      expect(cookies).toHaveLength(1);
      expect(cookies[0]!.value).toBe("new");
    });
  });

  describe("cookiesToHeader", () => {
    it("should format cookies as header string", () => {
      const header = cookiesToHeader([
        { name: "a", value: "1", domain: "x.com", path: "/" },
        { name: "b", value: "2", domain: "x.com", path: "/" },
      ]);
      expect(header).toBe("a=1; b=2");
    });

    it("should return empty string for no cookies", () => {
      expect(cookiesToHeader([])).toBe("");
    });
  });

  describe("parseSetCookie", () => {
    it("should parse Set-Cookie header", () => {
      const cookie = parseSetCookie(
        "session=abc123; Path=/; HttpOnly; Secure; SameSite=Lax",
        "example.com",
      );
      expect(cookie.name).toBe("session");
      expect(cookie.value).toBe("abc123");
      expect(cookie.httpOnly).toBe(true);
      expect(cookie.secure).toBe(true);
      expect(cookie.sameSite).toBe("Lax");
    });

    it("should parse cookie with domain override", () => {
      const cookie = parseSetCookie(
        "id=42; Domain=.example.com; Path=/app",
        "sub.example.com",
      );
      expect(cookie.domain).toBe(".example.com");
      expect(cookie.path).toBe("/app");
    });

    it("should use default domain when not specified", () => {
      const cookie = parseSetCookie("simple=yes", "mysite.com");
      expect(cookie.domain).toBe("mysite.com");
      expect(cookie.path).toBe("/");
    });
  });

  describe("exportCookiesTxt", () => {
    it("should export in Netscape format", () => {
      const txt = exportCookiesTxt([
        {
          name: "test",
          value: "val",
          domain: ".example.com",
          path: "/",
          secure: true,
        },
      ]);
      expect(txt).toContain("# Netscape HTTP Cookie File");
      expect(txt).toContain(".example.com");
      expect(txt).toContain("TRUE");
    });

    it("should handle non-secure cookies", () => {
      const txt = exportCookiesTxt([
        { name: "x", value: "y", domain: "test.com", path: "/" },
      ]);
      // Domain does not start with '.', so subdomain flag is FALSE
      expect(txt).toContain("FALSE");
    });

    it("should handle empty cookie list", () => {
      const txt = exportCookiesTxt([]);
      expect(txt).toBe("# Netscape HTTP Cookie File");
    });
  });
});
