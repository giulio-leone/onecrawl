/**
 * Popup OAuth Login Adapter
 *
 * Opens a compact popup window (not headless) showing only the service login page.
 * The user completes login manually (including 2FA, captcha, etc.).
 * Cookies are saved to the profile's user-data-dir for future headless use.
 *
 * Priority 2 — user-friendly, works with any auth flow.
 */

import type {
  BrowserProfile,
  LoginResult,
  SocialService,
} from "../../domain/browser-profile.js";
import type { LoginOptions } from "../../ports/login.port.js";
import { SERVICE_LOGIN_URLS } from "../../ports/login.port.js";
import { BaseLoginAdapter } from "./base.adapter.js";

const ALL_SERVICES: SocialService[] = [
  "x", "linkedin", "instagram", "facebook", "threads",
  "tiktok", "youtube", "pinterest", "reddit",
];

export class PopupOAuthAdapter extends BaseLoginAdapter {
  readonly name = "popup-oauth";
  readonly priority = 2;

  canHandle(_service: SocialService): boolean {
    return true; // Works for all services
  }

  async login(
    profile: BrowserProfile,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult> {
    const start = Date.now();
    const timeoutMs = options?.timeoutMs ?? 120_000; // 2 minutes default

    let browser = null;
    try {
      // Always launch headed for popup login
      browser = await this.launchBrowser(profile, false);
      const context = await this.createStealthContext(browser);
      const page = await context.newPage();

      // Set a compact viewport (popup style)
      await page.setViewportSize({ width: 480, height: 720 });

      // Notify user that interaction is required
      options?.onInteractionRequired?.({
        type: "oauth_prompt",
        message: `Completa il login su ${service}. La finestra si chiuderà automaticamente.`,
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
        // Take screenshot to show what happened
        const screenshot = await this.takeScreenshot(page);
        await page.close();
        await context.close();

        return {
          success: false,
          method: "popup",
          error: `Login timeout dopo ${Math.floor(timeoutMs / 1000)}s. L'utente non ha completato il login.`,
          requiresInteraction: true,
          screenshotBase64: screenshot,
          durationMs: Date.now() - start,
        };
      }

      // Small delay to ensure cookies are flushed
      await new Promise((r) => setTimeout(r, 2000));
      await page.close();
      await context.close();

      return {
        success: true,
        method: "popup",
        durationMs: Date.now() - start,
      };
    } catch (err) {
      return {
        success: false,
        method: "popup",
        error: `Popup login failed: ${err instanceof Error ? err.message : "unknown"}`,
        durationMs: Date.now() - start,
      };
    } finally {
      await browser?.close().catch(() => {});
    }
  }
}
