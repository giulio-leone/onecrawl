/**
 * Playwright Page Handle wrapper
 */

import type { Page } from "playwright";
import type { PageHandle } from "../../ports/index.js";

/** Wrapper for Playwright Page implementing PageHandle port. */
export class PlaywrightPageHandle implements PageHandle {
  constructor(readonly page: Page) {}

  url(): string {
    return this.page.url();
  }

  async title(): Promise<string> {
    return this.page.title();
  }

  async content(): Promise<string> {
    return this.page.content();
  }

  async evaluate<T>(script: string | (() => T)): Promise<T> {
    if (typeof script === "function") {
      return this.page.evaluate(script);
    }
    return this.page.evaluate(script);
  }

  async waitForSelector(
    selector: string,
    options?: { timeout?: number },
  ): Promise<void> {
    await this.page.waitForSelector(selector, options);
  }

  async waitForNavigation(options?: { timeout?: number }): Promise<void> {
    await this.page.waitForLoadState("networkidle", options);
  }

  async click(selector: string): Promise<void> {
    await this.page.click(selector);
  }

  async type(selector: string, text: string): Promise<void> {
    await this.page.fill(selector, text);
  }

  async screenshot(options?: { fullPage?: boolean }): Promise<Buffer> {
    return this.page.screenshot({ fullPage: options?.fullPage });
  }

  async close(): Promise<void> {
    await this.page.close();
  }
}
