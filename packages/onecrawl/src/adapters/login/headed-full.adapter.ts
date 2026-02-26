/**
 * Headed Full Login Adapter
 *
 * Last resort: opens a full Chrome window with complete UI.
 * The user interacts with Chrome directly to complete login.
 * Cookies persist in the profile's user-data-dir.
 *
 * Priority 4 — most reliable but least automated.
 */

import type {
  BrowserProfile,
  LoginResult,
  SocialService,
} from "../../domain/browser-profile.js";
import type { LoginOptions } from "../../ports/login.port.js";
import { SERVICE_LOGIN_URLS } from "../../ports/login.port.js";
import { BaseLoginAdapter } from "./base.adapter.js";

export class HeadedFullAdapter extends BaseLoginAdapter {
  readonly name = "headed-full";
  readonly priority = 4;

  canHandle(_service: SocialService): boolean {
    return true; // Works for everything
  }

  async login(
    profile: BrowserProfile,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult> {
    const start = Date.now();
    const timeoutMs = options?.timeoutMs ?? 180_000; // 3 minutes for manual login

    let context = null;
    try {
      // Always launch headed — this is the full-UI fallback
      context = await this.launchPersistentContext(profile, false);
      const page = await context.newPage();

      // Use a comfortable viewport
      await page.setViewportSize({ width: 1280, height: 800 });

      // Notify user
      options?.onInteractionRequired?.({
        type: "manual_login",
        message: `Chrome aperto. Completa il login su ${service} manualmente. Hai ${Math.floor(timeoutMs / 1000)}s.`,
        remainingSeconds: Math.floor(timeoutMs / 1000),
      });

      // Navigate to login page
      const loginUrl = SERVICE_LOGIN_URLS[service];
      await page.goto(loginUrl, {
        waitUntil: "domcontentloaded",
        timeout: 15_000,
      });

      // Wait for user to complete login
      const loggedIn = await this.waitForLogin(page, service, timeoutMs);

      if (!loggedIn) {
        const screenshot = await this.takeScreenshot(page);
        await page.close();

        return {
          success: false,
          method: "headed",
          error: `Timeout: login manuale non completato in ${Math.floor(timeoutMs / 1000)}s`,
          requiresInteraction: true,
          screenshotBase64: screenshot,
          durationMs: Date.now() - start,
        };
      }

      // Wait for cookies to persist
      await new Promise((r) => setTimeout(r, 3000));
      await page.close();

      return {
        success: true,
        method: "headed",
        durationMs: Date.now() - start,
      };
    } catch (err) {
      return {
        success: false,
        method: "headed",
        error: `Headed login failed: ${err instanceof Error ? err.message : "unknown"}`,
        durationMs: Date.now() - start,
      };
    } finally {
      await context?.close().catch(() => {});
      this.context = null;
    }
  }
}
