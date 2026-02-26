/**
 * Web Actions — Zod request schema validation tests.
 * Tests that route schemas accept valid requests and reject invalid ones.
 */

import { describe, it, expect } from "vitest";
import {
  navigateSchema,
  clickSchema,
  typeSchema,
  pressKeySchema,
  screenshotSchema,
  uploadSchema,
  waitSchema,
  extractSchema,
  evaluateSchema,
  getCookiesSchema,
  setCookiesSchema,
  humanWarmupSchema,
  humanClickSchema,
  humanTypeSchema,
} from "../../src/web-actions/routes.js";

// ── navigateSchema ─────────────────────────────────────────────────────────

describe("navigateSchema", () => {
  it("accepts valid request with defaults", () => {
    const r = navigateSchema.parse({
      profileId: "profile-1",
      url: "https://example.com",
    });
    expect(r.waitUntil).toBe("domcontentloaded");
    expect(r.timeout).toBe(30_000);
    expect(r.headless).toBe(true);
  });

  it("accepts full request", () => {
    const r = navigateSchema.parse({
      profileId: "p",
      url: "https://x.com",
      waitUntil: "networkidle",
      timeout: 60_000,
      headless: false,
      cdpUrl: "http://127.0.0.1:9222",
    });
    expect(r.waitUntil).toBe("networkidle");
    expect(r.headless).toBe(false);
    expect(r.cdpUrl).toBe("http://127.0.0.1:9222");
  });

  it("rejects empty profileId", () => {
    expect(() =>
      navigateSchema.parse({ profileId: "", url: "https://x.com" }),
    ).toThrow();
  });

  it("rejects invalid url", () => {
    expect(() =>
      navigateSchema.parse({ profileId: "p", url: "not-a-url" }),
    ).toThrow();
  });

  it("rejects invalid waitUntil", () => {
    expect(() =>
      navigateSchema.parse({
        profileId: "p",
        url: "https://x.com",
        waitUntil: "ready",
      }),
    ).toThrow();
  });

  it("rejects non-positive timeout", () => {
    expect(() =>
      navigateSchema.parse({
        profileId: "p",
        url: "https://x.com",
        timeout: 0,
      }),
    ).toThrow();
    expect(() =>
      navigateSchema.parse({
        profileId: "p",
        url: "https://x.com",
        timeout: -1,
      }),
    ).toThrow();
  });

  it("rejects invalid cdpUrl", () => {
    expect(() =>
      navigateSchema.parse({
        profileId: "p",
        url: "https://x.com",
        cdpUrl: "not-url",
      }),
    ).toThrow();
  });
});

// ── clickSchema ────────────────────────────────────────────────────────────

describe("clickSchema", () => {
  it("accepts click by selector", () => {
    const r = clickSchema.parse({ profileId: "p", selector: "#btn" });
    expect(r.button).toBe("left");
    expect(r.clickCount).toBe(1);
    expect(r.force).toBe(false);
  });

  it("accepts click by text", () => {
    const r = clickSchema.parse({ profileId: "p", text: "Submit" });
    expect(r.text).toBe("Submit");
  });

  it("accepts all button types", () => {
    for (const button of ["left", "right", "middle"] as const) {
      const r = clickSchema.parse({ profileId: "p", selector: "a", button });
      expect(r.button).toBe(button);
    }
  });

  it("accepts double/triple click", () => {
    const r = clickSchema.parse({
      profileId: "p",
      selector: "p",
      clickCount: 3,
    });
    expect(r.clickCount).toBe(3);
  });

  it("rejects clickCount > 3", () => {
    expect(() =>
      clickSchema.parse({ profileId: "p", selector: "a", clickCount: 4 }),
    ).toThrow();
  });

  it("rejects empty selector", () => {
    expect(() =>
      clickSchema.parse({ profileId: "p", selector: "" }),
    ).toThrow();
  });
});

// ── typeSchema ─────────────────────────────────────────────────────────────

describe("typeSchema", () => {
  it("accepts valid type request", () => {
    const r = typeSchema.parse({
      profileId: "p",
      selector: "#input",
      text: "hello",
    });
    expect(r.clear).toBe(false);
    expect(r.delay).toBe(50);
  });

  it("accepts empty text (clear field)", () => {
    const r = typeSchema.parse({
      profileId: "p",
      selector: "#input",
      text: "",
      clear: true,
    });
    expect(r.text).toBe("");
    expect(r.clear).toBe(true);
  });

  it("rejects delay > 500", () => {
    expect(() =>
      typeSchema.parse({
        profileId: "p",
        selector: "x",
        text: "t",
        delay: 501,
      }),
    ).toThrow();
  });

  it("rejects negative delay", () => {
    expect(() =>
      typeSchema.parse({
        profileId: "p",
        selector: "x",
        text: "t",
        delay: -1,
      }),
    ).toThrow();
  });
});

// ── pressKeySchema ─────────────────────────────────────────────────────────

describe("pressKeySchema", () => {
  it("accepts valid key press", () => {
    const r = pressKeySchema.parse({ profileId: "p", key: "Enter" });
    expect(r.key).toBe("Enter");
  });

  it("rejects empty key", () => {
    expect(() => pressKeySchema.parse({ profileId: "p", key: "" })).toThrow();
  });
});

// ── screenshotSchema ───────────────────────────────────────────────────────

describe("screenshotSchema", () => {
  it("accepts minimal", () => {
    const r = screenshotSchema.parse({ profileId: "p" });
    expect(r.fullPage).toBe(false);
    expect(r.selector).toBeUndefined();
  });

  it("accepts full page with selector", () => {
    const r = screenshotSchema.parse({
      profileId: "p",
      fullPage: true,
      selector: "#main",
    });
    expect(r.fullPage).toBe(true);
    expect(r.selector).toBe("#main");
  });
});

// ── uploadSchema ───────────────────────────────────────────────────────────

describe("uploadSchema", () => {
  it("accepts valid upload", () => {
    const r = uploadSchema.parse({
      profileId: "p",
      selector: "input[type=file]",
      filePath: "/tmp/photo.jpg",
    });
    expect(r.timeout).toBe(10_000);
  });

  it("rejects empty selector", () => {
    expect(() =>
      uploadSchema.parse({ profileId: "p", selector: "", filePath: "f" }),
    ).toThrow();
  });

  it("rejects empty filePath", () => {
    expect(() =>
      uploadSchema.parse({ profileId: "p", selector: "s", filePath: "" }),
    ).toThrow();
  });
});

// ── waitSchema ─────────────────────────────────────────────────────────────

describe("waitSchema", () => {
  it("accepts wait for selector", () => {
    const r = waitSchema.parse({ profileId: "p", selector: ".loaded" });
    expect(r.state).toBe("visible");
    expect(r.timeout).toBe(30_000);
  });

  it("accepts wait for URL", () => {
    const r = waitSchema.parse({
      profileId: "p",
      url: "https://example.com/done",
    });
    expect(r.url).toBe("https://example.com/done");
  });

  it("accepts all states", () => {
    for (const state of ["attached", "detached", "visible", "hidden"] as const) {
      const r = waitSchema.parse({ profileId: "p", selector: "x", state });
      expect(r.state).toBe(state);
    }
  });

  it("rejects invalid state", () => {
    expect(() =>
      waitSchema.parse({ profileId: "p", selector: "x", state: "ready" }),
    ).toThrow();
  });
});

// ── extractSchema ──────────────────────────────────────────────────────────

describe("extractSchema", () => {
  it("accepts single element extraction", () => {
    const r = extractSchema.parse({ profileId: "p", selector: "h1" });
    expect(r.all).toBe(false);
  });

  it("accepts multi-element extraction with attribute", () => {
    const r = extractSchema.parse({
      profileId: "p",
      selector: "a",
      attribute: "href",
      all: true,
    });
    expect(r.all).toBe(true);
    expect(r.attribute).toBe("href");
  });
});

// ── evaluateSchema ─────────────────────────────────────────────────────────

describe("evaluateSchema", () => {
  it("accepts valid script", () => {
    const r = evaluateSchema.parse({
      profileId: "p",
      script: "document.title",
    });
    expect(r.script).toBe("document.title");
  });

  it("rejects empty script", () => {
    expect(() =>
      evaluateSchema.parse({ profileId: "p", script: "" }),
    ).toThrow();
  });
});

// ── Cookie schemas ─────────────────────────────────────────────────────────

describe("getCookiesSchema", () => {
  it("accepts without urls", () => {
    const r = getCookiesSchema.parse({ profileId: "p" });
    expect(r.urls).toBeUndefined();
  });

  it("accepts with urls", () => {
    const r = getCookiesSchema.parse({
      profileId: "p",
      urls: ["https://example.com"],
    });
    expect(r.urls).toHaveLength(1);
  });

  it("rejects invalid urls", () => {
    expect(() =>
      getCookiesSchema.parse({ profileId: "p", urls: ["bad"] }),
    ).toThrow();
  });
});

describe("setCookiesSchema", () => {
  it("accepts valid cookies", () => {
    const r = setCookiesSchema.parse({
      profileId: "p",
      cookies: [{ name: "session", value: "abc123" }],
    });
    expect(r.cookies).toHaveLength(1);
  });

  it("accepts cookie with all options", () => {
    const r = setCookiesSchema.parse({
      profileId: "p",
      cookies: [
        {
          name: "token",
          value: "xyz",
          domain: ".example.com",
          path: "/",
          expires: 1700000000,
          httpOnly: true,
          secure: true,
          sameSite: "Strict",
        },
      ],
    });
    expect(r.cookies[0]!.sameSite).toBe("Strict");
  });

  it("rejects empty cookies array", () => {
    expect(() =>
      setCookiesSchema.parse({ profileId: "p", cookies: [] }),
    ).toThrow();
  });

  it("rejects invalid sameSite", () => {
    expect(() =>
      setCookiesSchema.parse({
        profileId: "p",
        cookies: [{ name: "a", value: "b", sameSite: "Invalid" }],
      }),
    ).toThrow();
  });
});

// ── Human Behavior schemas ─────────────────────────────────────────────────

describe("humanWarmupSchema", () => {
  it("accepts valid request", () => {
    const r = humanWarmupSchema.parse({ profileId: "p" });
    expect(r.profileId).toBe("p");
  });

  it("rejects empty profileId", () => {
    expect(() => humanWarmupSchema.parse({ profileId: "" })).toThrow();
  });
});

describe("humanClickSchema", () => {
  it("accepts valid request", () => {
    const r = humanClickSchema.parse({ profileId: "p", selector: "#btn" });
    expect(r.selector).toBe("#btn");
  });

  it("rejects missing selector", () => {
    expect(() => humanClickSchema.parse({ profileId: "p" })).toThrow();
  });
});

describe("humanTypeSchema", () => {
  it("accepts valid request", () => {
    const r = humanTypeSchema.parse({
      profileId: "p",
      selector: "#input",
      text: "hello world",
    });
    expect(r.text).toBe("hello world");
  });

  it("rejects empty text", () => {
    expect(() =>
      humanTypeSchema.parse({ profileId: "p", selector: "s", text: "" }),
    ).toThrow();
  });
});
