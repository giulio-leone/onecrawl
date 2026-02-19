/**
 * Passkey Login Adapter
 *
 * Uses CDP Virtual Authenticator to perform passkey-based login.
 * Priority 1 (tried first) — fastest and most seamless.
 *
 * NOTE: This is a forward-looking adapter. Full passkey support depends on:
 * - The service supporting WebAuthn/FIDO2 login
 * - The user having a passkey registered
 * - iCloud Keychain / password manager integration (future)
 *
 * Currently: attempts passkey flow, returns graceful fallback if not available.
 */

import type { CDPSession, Page } from "playwright-core";
import type {
  BrowserProfile,
  LoginResult,
  SocialService,
} from "../../domain/browser-profile.js";
import type { LoginOptions } from "../../ports/login.port.js";
import { SERVICE_LOGIN_URLS } from "../../ports/login.port.js";
import { BaseLoginAdapter } from "./base.adapter.js";

/** Services known to support passkey/WebAuthn login. */
const PASSKEY_SUPPORTED: SocialService[] = ["x", "linkedin", "youtube"];

export class PasskeyLoginAdapter extends BaseLoginAdapter {
  readonly name = "passkey";
  readonly priority = 1;

  canHandle(service: SocialService): boolean {
    return PASSKEY_SUPPORTED.includes(service);
  }

  async login(
    profile: BrowserProfile,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult> {
    const start = Date.now();
    const headless = options?.headless ?? profile.headless;
    let browser = null;

    try {
      browser = await this.launchBrowser(profile, headless);
      const context = await this.createStealthContext(browser);
      const page = await context.newPage();

      // Set up virtual authenticator via CDP
      const cdpSession = await context.newCDPSession(page);
      await this.setupVirtualAuthenticator(cdpSession);

      // Navigate to login page
      const loginUrl = SERVICE_LOGIN_URLS[service];
      await page.goto(loginUrl, {
        waitUntil: "domcontentloaded",
        timeout: 15_000,
      });

      // Try to find and click passkey/WebAuthn option
      const passkeyClicked = await this.clickPasskeyOption(page, service);
      if (!passkeyClicked) {
        return {
          success: false,
          method: "passkey",
          error: "Passkey option not found on login page",
          durationMs: Date.now() - start,
        };
      }

      // Wait for login to complete
      const loggedIn = await this.waitForLogin(
        page,
        service,
        options?.timeoutMs ?? 10_000,
      );

      await page.close();
      await context.close();

      return {
        success: loggedIn,
        method: "passkey",
        error: loggedIn ? undefined : "Passkey authentication timed out",
        durationMs: Date.now() - start,
      };
    } catch (err) {
      return {
        success: false,
        method: "passkey",
        error: `Passkey login failed: ${err instanceof Error ? err.message : "unknown"}`,
        durationMs: Date.now() - start,
      };
    } finally {
      await browser?.close().catch(() => {});
    }
  }

  /** Set up a CDP Virtual Authenticator for passkey-based login. */
  private async setupVirtualAuthenticator(cdp: CDPSession): Promise<void> {
    await cdp.send("WebAuthn.enable" as never);
    await cdp.send("WebAuthn.addVirtualAuthenticator" as never, {
      options: {
        protocol: "ctap2",
        transport: "internal",
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
      },
    } as never);
  }

  /** Try to find and click the passkey/WebAuthn login option. */
  private async clickPasskeyOption(
    page: Page,
    service: SocialService,
  ): Promise<boolean> {
    const selectors: Record<string, string[]> = {
      x: [
        'button:has-text("Sign in with a passkey")',
        '[data-testid="passkey_button"]',
        'button:has-text("Passkey")',
      ],
      linkedin: [
        'button:has-text("Sign in with passkey")',
        'button:has-text("Passkey")',
      ],
      youtube: [
        'button:has-text("Use your passkey")',
        'button:has-text("Passkey")',
      ],
    };

    const candidates = selectors[service] ?? [];
    for (const sel of candidates) {
      try {
        const el = await page.$(sel);
        if (el) {
          await el.click();
          return true;
        }
      } catch {
        continue;
      }
    }
    return false;
  }
}
