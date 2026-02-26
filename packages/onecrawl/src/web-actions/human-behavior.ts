/**
 * Human behavior simulation — makes browser automation indistinguishable
 * from real user interactions.
 *
 * Uses ghost-cursor-playwright for natural mouse movements (Bezier curves)
 * and adds random delays, scrolling, and viewport interactions.
 */

import { createCursor } from "ghost-cursor-playwright";
import type { Page } from "rebrowser-playwright";

// ── Types ────────────────────────────────────────────────────────────────────

/** Ghost cursor instance returned by createCursor */
export type HumanCursor = Awaited<ReturnType<typeof createCursor>>;

export interface HumanClickOptions {
  /** CSS selector to click */
  selector: string;
  /** Min/max ms to wait before moving mouse [min, max] */
  waitBeforeMove?: [number, number];
  /** Min/max ms to wait before clicking [min, max] */
  waitBeforeClick?: [number, number];
}

export interface HumanTypeOptions {
  /** CSS selector of the input element */
  selector: string;
  /** Text to type */
  text: string;
  /** Min/max ms delay between keystrokes [min, max] */
  keystrokeDelay?: [number, number];
}

// ── Constants ────────────────────────────────────────────────────────────────

const DEFAULT_WAIT_BEFORE_MOVE: [number, number] = [200, 600];
const DEFAULT_WAIT_BEFORE_CLICK: [number, number] = [100, 300];
const DEFAULT_KEYSTROKE_DELAY: [number, number] = [50, 150];

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Random integer in [min, max] range. */
function randomInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

/** Sleep for a random duration in [min, max] ms. */
function randomSleep(min: number, max: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, randomInt(min, max)));
}

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * Create a ghost-cursor instance attached to a Playwright page.
 * The cursor simulates human-like Bezier mouse movements.
 */
export async function createHumanCursor(page: Page): Promise<HumanCursor> {
  // Cast needed: rebrowser-playwright Page is structurally compatible with playwright-core Page
  return await createCursor(page as any);
}

/**
 * Perform a human-like click: move mouse with natural curve, pause, click.
 */
export async function humanClick(
  page: Page,
  cursor: HumanCursor,
  options: HumanClickOptions,
): Promise<void> {
  const waitMove = options.waitBeforeMove ?? DEFAULT_WAIT_BEFORE_MOVE;
  const waitClick = options.waitBeforeClick ?? DEFAULT_WAIT_BEFORE_CLICK;

  await randomSleep(waitMove[0], waitMove[1]);
  await cursor.actions.move(options.selector, {
    paddingPercentage: 10,
    waitForSelector: 5000,
  });
  await randomSleep(waitClick[0], waitClick[1]);
  await cursor.actions.click({ target: options.selector });
}

/**
 * Type text with human-like variable delays between keystrokes.
 */
export async function humanType(
  page: Page,
  options: HumanTypeOptions,
): Promise<void> {
  const delay = options.keystrokeDelay ?? DEFAULT_KEYSTROKE_DELAY;

  for (const char of options.text) {
    await page.keyboard.type(char, { delay: randomInt(delay[0], delay[1]) });
  }
}

/**
 * Simulate reading/browsing behavior before taking action:
 * random scroll, mouse movements, and pauses.
 */
export async function humanWarmup(
  page: Page,
  cursor: HumanCursor,
): Promise<void> {
  // Small random mouse movement in viewport
  const vw = 1280;
  const vh = 800;
  await cursor.actions.move({
    x: randomInt(100, vw - 100),
    y: randomInt(100, vh - 200),
  });
  await randomSleep(500, 1500);

  // Scroll down a bit (like reading the feed)
  await page.mouse.wheel(0, randomInt(100, 400));
  await randomSleep(800, 2000);

  // Scroll back up
  await page.mouse.wheel(0, -randomInt(50, 200));
  await randomSleep(300, 800);

  // Another mouse movement
  await cursor.actions.move({
    x: randomInt(200, vw - 200),
    y: randomInt(50, vh / 2),
  });
  await randomSleep(200, 500);
}

/**
 * Simulate random scroll and pause, as if reading content.
 */
export async function humanScroll(
  page: Page,
  options?: { scrollAmount?: [number, number]; pauseAfter?: [number, number] },
): Promise<void> {
  const amount = options?.scrollAmount ?? [150, 500];
  const pause = options?.pauseAfter ?? [500, 1500];

  await page.mouse.wheel(0, randomInt(amount[0], amount[1]));
  await randomSleep(pause[0], pause[1]);
}
