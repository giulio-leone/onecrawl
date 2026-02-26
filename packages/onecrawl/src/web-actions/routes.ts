/**
 * Web Action HTTP route handlers — generic browser automation primitives.
 *
 * Each action takes a profileId to identify the browser session,
 * then performs the requested operation via Playwright.
 */
import { z } from "zod";
import type { WebActionManager, WebActionError } from "./manager.js";
import { humanWarmup, humanClick, humanType } from "./human-behavior.js";

// ── Zod Schemas ──────────────────────────────────────────────────────────────

const profileIdSchema = z.string().min(1).max(200);

export const navigateSchema = z.object({
  profileId: profileIdSchema,
  url: z.string().url(),
  waitUntil: z
    .enum(["load", "domcontentloaded", "networkidle", "commit"])
    .default("domcontentloaded"),
  timeout: z.number().int().positive().default(30_000),
  headless: z.boolean().default(true),
  cdpUrl: z.string().url().optional(),
});

export const clickSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1).optional(),
  text: z.string().min(1).optional(),
  button: z.enum(["left", "right", "middle"]).default("left"),
  clickCount: z.number().int().min(1).max(3).default(1),
  timeout: z.number().int().positive().default(10_000),
  force: z.boolean().default(false),
});

export const typeSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1),
  text: z.string(),
  clear: z.boolean().default(false),
  delay: z.number().int().min(0).max(500).default(50),
  timeout: z.number().int().positive().default(10_000),
});

export const pressKeySchema = z.object({
  profileId: profileIdSchema,
  key: z.string().min(1), // e.g. "Enter", "Tab", "Escape", "ArrowDown"
});

export const screenshotSchema = z.object({
  profileId: profileIdSchema,
  fullPage: z.boolean().default(false),
  selector: z.string().optional(),
});

export const uploadSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1),
  filePath: z.string().min(1),
  timeout: z.number().int().positive().default(10_000),
});

export const waitSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1).optional(),
  state: z.enum(["attached", "detached", "visible", "hidden"]).default("visible"),
  timeout: z.number().int().positive().default(30_000),
  url: z.string().optional(),
});

export const extractSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1),
  attribute: z.string().optional(),
  all: z.boolean().default(false),
  timeout: z.number().int().positive().default(10_000),
});

export const evaluateSchema = z.object({
  profileId: profileIdSchema,
  script: z.string().min(1),
});

export const getCookiesSchema = z.object({
  profileId: profileIdSchema,
  urls: z.array(z.string().url()).optional(),
});

const cookieSchema = z.object({
  name: z.string(),
  value: z.string(),
  domain: z.string().optional(),
  path: z.string().optional(),
  expires: z.number().optional(),
  httpOnly: z.boolean().optional(),
  secure: z.boolean().optional(),
  sameSite: z.enum(["Strict", "Lax", "None"]).optional(),
});

export const setCookiesSchema = z.object({
  profileId: profileIdSchema,
  cookies: z.array(cookieSchema).min(1),
});

export const humanWarmupSchema = z.object({
  profileId: profileIdSchema,
});

export const humanClickSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1),
});

export const humanTypeSchema = z.object({
  profileId: profileIdSchema,
  selector: z.string().min(1),
  text: z.string().min(1),
});

// ── Route Handlers ───────────────────────────────────────────────────────────

function actionError(
  code: WebActionError["code"],
  message: string,
  screenshot?: string,
): WebActionError {
  return { code, message, screenshot };
}

export async function handleNavigate(body: unknown, wam: WebActionManager) {
  const parsed = navigateSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, url, waitUntil, timeout, headless, cdpUrl } = parsed.data;
  try {
    const page = await wam.getPage(profileId, { headless, cdpUrl });
    const response = await page.goto(url, { waitUntil, timeout });
    return {
      status: 200,
      body: {
        url: page.url(),
        title: await page.title(),
        status: response?.status() ?? null,
      },
    };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("NAVIGATION_ERROR", (err as Error).message, screenshot),
    };
  }
}

export async function handleClick(body: unknown, wam: WebActionManager) {
  const parsed = clickSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector, text, button, clickCount, timeout, force } = parsed.data;
  if (!selector && !text)
    return { status: 400, body: { error: "Either 'selector' or 'text' is required" } };

  try {
    const page = await wam.getPage(profileId);

    if (text) {
      await page.getByText(text, { exact: false }).first().click({ button, clickCount, timeout, force });
    } else {
      await page.locator(selector!).first().click({ button, clickCount, timeout, force });
    }

    return { status: 200, body: { clicked: true, selector: selector ?? `text="${text}"` } };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("ELEMENT_NOT_FOUND", (err as Error).message, screenshot),
    };
  }
}

export async function handleType(body: unknown, wam: WebActionManager) {
  const parsed = typeSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector, text, clear, delay, timeout } = parsed.data;
  try {
    const page = await wam.getPage(profileId);
    const locator = page.locator(selector).first();

    // Detect element type to choose the right input method
    const tagName = await locator.evaluate((el) => el.tagName.toLowerCase());
    const isInput = tagName === "input" || tagName === "textarea" || tagName === "select";

    if (isInput) {
      // For standard form elements, use fill() — fires proper React/Vue events
      if (clear) await locator.fill("", { timeout });
      await locator.fill(text, { timeout });
    } else {
      // For contentEditable divs (tweet composer, LinkedIn editor), use insertText
      if (clear) await locator.fill("", { timeout });
      await locator.click({ timeout });
      if (delay > 0) {
        await page.keyboard.type(text, { delay });
      } else {
        await page.keyboard.insertText(text);
      }
    }

    return { status: 200, body: { typed: true, selector, length: text.length } };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("ELEMENT_NOT_FOUND", (err as Error).message, screenshot),
    };
  }
}

export async function handlePressKey(body: unknown, wam: WebActionManager) {
  const parsed = pressKeySchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, key } = parsed.data;
  try {
    const page = await wam.getPage(profileId);
    await page.keyboard.press(key);
    return { status: 200, body: { pressed: true, key } };
  } catch (err) {
    return {
      status: 422,
      body: actionError("ELEMENT_NOT_FOUND", (err as Error).message),
    };
  }
}

export async function handleGetCookies(body: unknown, wam: WebActionManager) {
  const parsed = getCookiesSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, urls } = parsed.data;
  try {
    const page = await wam.getPage(profileId);
    const cookies = urls
      ? await page.context().cookies(urls)
      : await page.context().cookies();
    return { status: 200, body: { cookies, count: cookies.length } };
  } catch (err) {
    return {
      status: 422,
      body: actionError("UNKNOWN", (err as Error).message),
    };
  }
}

export async function handleSetCookies(body: unknown, wam: WebActionManager) {
  const parsed = setCookiesSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, cookies } = parsed.data;
  try {
    const page = await wam.getPage(profileId);
    await page.context().addCookies(cookies);
    return { status: 200, body: { added: cookies.length } };
  } catch (err) {
    return {
      status: 422,
      body: actionError("UNKNOWN", (err as Error).message),
    };
  }
}

export async function handleScreenshot(body: unknown, wam: WebActionManager) {
  const parsed = screenshotSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, fullPage, selector } = parsed.data;
  try {
    const base64 = await wam.captureScreenshot(profileId, {
      fullPage,
      selector,
    });
    return { status: 200, body: { screenshot: base64, format: "png" } };
  } catch (err) {
    return {
      status: 422,
      body: actionError("UNKNOWN", (err as Error).message),
    };
  }
}

export async function handleUpload(body: unknown, wam: WebActionManager) {
  const parsed = uploadSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector, filePath, timeout } = parsed.data;
  try {
    const page = await wam.getPage(profileId);
    const fileInput = page.locator(selector).first();
    await fileInput.setInputFiles(filePath, { timeout });

    return { status: 200, body: { uploaded: true, selector, filePath } };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("UPLOAD_ERROR", (err as Error).message, screenshot),
    };
  }
}

export async function handleWait(body: unknown, wam: WebActionManager) {
  const parsed = waitSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector, state, timeout, url } = parsed.data;
  try {
    const page = await wam.getPage(profileId);

    if (url) {
      await page.waitForURL(url, { timeout });
    } else if (selector) {
      await page.locator(selector).waitFor({ state, timeout });
    } else {
      await page.waitForTimeout(Math.min(timeout, 5000));
    }

    return { status: 200, body: { waited: true, currentUrl: page.url() } };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("TIMEOUT", (err as Error).message, screenshot),
    };
  }
}

export async function handleExtract(body: unknown, wam: WebActionManager) {
  const parsed = extractSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector, attribute, all, timeout } = parsed.data;
  try {
    const page = await wam.getPage(profileId);

    if (all) {
      const locators = page.locator(selector);
      const count = await locators.count();
      const results: Array<{ text: string; attribute?: string }> = [];

      for (let i = 0; i < Math.min(count, 100); i++) {
        const el = locators.nth(i);
        const text = (await el.textContent({ timeout })) ?? "";
        const attr = attribute ? await el.getAttribute(attribute) : undefined;
        results.push({ text: text.trim(), attribute: attr ?? undefined });
      }

      return { status: 200, body: { results, count } };
    }

    const el = page.locator(selector).first();
    const text = (await el.textContent({ timeout })) ?? "";
    const attr = attribute ? await el.getAttribute(attribute) : undefined;

    return {
      status: 200,
      body: { text: text.trim(), attribute: attr ?? undefined, selector },
    };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("ELEMENT_NOT_FOUND", (err as Error).message, screenshot),
    };
  }
}

export async function handleEvaluate(body: unknown, wam: WebActionManager) {
  const parsed = evaluateSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, script } = parsed.data;
  try {
    const page = await wam.getPage(profileId);
    const result = await page.evaluate(script);
    return { status: 200, body: { result } };
  } catch (err) {
    const screenshot = await wam.errorScreenshot(profileId);
    return {
      status: 422,
      body: actionError("EVALUATION_ERROR", (err as Error).message, screenshot),
    };
  }
}

export function handleListSessions(wam: WebActionManager) {
  const sessions = wam.listSessions();
  return { status: 200, body: { sessions, count: sessions.length } };
}

export async function handleCloseSession(profileId: string, wam: WebActionManager) {
  const closed = await wam.closeSession(profileId);
  if (!closed) {
    return { status: 404, body: { error: "Session not found" } };
  }
  return { status: 200, body: { closed: true, profileId } };
}

// ── CDP Connect ─────────────────────────────────────────────────────────────

const connectCdpSchema = z.object({
  profileId: profileIdSchema,
  cdpUrl: z.string().url().default("http://127.0.0.1:9222"),
});

/**
 * POST /web/connect — attach to an already-running Chrome via CDP.
 * The user must start Chrome with `--remote-debugging-port=9222`.
 */
export async function handleConnectCDP(body: unknown, wam: WebActionManager) {
  const parsed = connectCdpSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, cdpUrl } = parsed.data;
  try {
    const session = await wam.connectViaCDP(profileId, cdpUrl);
    return {
      status: 200,
      body: {
        connected: true,
        profileId,
        cdpUrl,
        currentUrl: session.page.isClosed() ? null : session.page.url(),
        pagesCount: session.context.pages().length,
      },
    };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { status: 502, body: { error: "CDP connection failed", message: msg } };
  }
}

// ── Human Behavior Routes ────────────────────────────────────────────────────

export async function handleHumanWarmup(body: unknown, wam: WebActionManager) {
  const parsed = humanWarmupSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId } = parsed.data;
  try {
    const session = await wam.getOrCreateSession(profileId);
    if (!session.cursor) {
      return { status: 200, body: { warmup: false, reason: "cursor not available" } };
    }
    await humanWarmup(session.page, session.cursor);
    return { status: 200, body: { warmup: true } };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { status: 500, body: { error: "Warmup failed", message: msg } };
  }
}

export async function handleHumanClick(body: unknown, wam: WebActionManager) {
  const parsed = humanClickSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector } = parsed.data;
  try {
    const session = await wam.getOrCreateSession(profileId);
    if (!session.cursor) {
      // Fallback to standard Playwright click
      await session.page.click(selector);
      return { status: 200, body: { clicked: true, humanLike: false } };
    }
    await humanClick(session.page, session.cursor, { selector });
    return { status: 200, body: { clicked: true, humanLike: true } };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { status: 500, body: { error: "Click failed", message: msg } };
  }
}

export async function handleHumanType(body: unknown, wam: WebActionManager) {
  const parsed = humanTypeSchema.safeParse(body);
  if (!parsed.success)
    return { status: 400, body: { error: "Invalid request", details: parsed.error.flatten() } };

  const { profileId, selector, text } = parsed.data;
  try {
    const session = await wam.getOrCreateSession(profileId);
    // Focus the element first
    await session.page.click(selector);
    await humanType(session.page, { selector, text });
    return { status: 200, body: { typed: true, humanLike: true, length: text.length } };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { status: 500, body: { error: "Type failed", message: msg } };
  }
}
