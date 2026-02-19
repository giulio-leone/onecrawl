/**
 * Headless Form Fill Login Adapter
 *
 * Attempts to log in by filling the login form headlessly.
 * If captcha/2FA is detected, takes a screenshot and notifies the user.
 *
 * Priority 3 — automated but fragile (services change form structure).
 */

import type { Page } from "playwright-core";
import type {
  BrowserProfile,
  LoginResult,
  SocialService,
} from "../../domain/browser-profile.js";
import type { LoginOptions } from "../../ports/login.port.js";
import { SERVICE_LOGIN_URLS } from "../../ports/login.port.js";
import { BaseLoginAdapter } from "./base.adapter.js";
import { getRandomDelay } from "../../utils/stealth.js";

/** Service-specific form field selectors and flow. */
interface LoginFormSpec {
  usernameSelector: string;
  passwordSelector: string;
  submitSelector: string;
  /** Some services have a 2-step flow (enter email → next → enter password). */
  nextButtonSelector?: string;
  /** Selectors that indicate captcha/verification challenge. */
  captchaSelectors: string[];
  /** Selectors that indicate 2FA prompt. */
  twoFaSelectors: string[];
}

const FORM_SPECS: Partial<Record<SocialService, LoginFormSpec>> = {
  x: {
    usernameSelector: 'input[autocomplete="username"], input[name="text"]',
    passwordSelector: 'input[name="password"], input[type="password"]',
    submitSelector: '[data-testid="LoginForm_Login_Button"], button[type="submit"]',
    nextButtonSelector: '[data-testid="LoginForm_Next_Button"]',
    captchaSelectors: ['[data-testid="LoginFlow_Challenge"]', "iframe[src*='captcha']"],
    twoFaSelectors: ['[data-testid="LoginFlow_Challenge_Code"]', 'input[data-testid="ocfEnterTextTextInput"]'],
  },
  linkedin: {
    usernameSelector: "#username",
    passwordSelector: "#password",
    submitSelector: 'button[type="submit"]',
    captchaSelectors: ["#captcha-internal", "iframe[src*='captcha']"],
    twoFaSelectors: ['input[name="pin"]', "#input__email_verification_pin"],
  },
  instagram: {
    usernameSelector: 'input[name="username"]',
    passwordSelector: 'input[name="password"]',
    submitSelector: 'button[type="submit"]',
    captchaSelectors: ["iframe[src*='captcha']", ".coreSpriteVerificationCode"],
    twoFaSelectors: ['input[name="verificationCode"]', 'input[aria-label="Security Code"]'],
  },
  facebook: {
    usernameSelector: "#email",
    passwordSelector: "#pass",
    submitSelector: 'button[name="login"]',
    captchaSelectors: ["iframe[src*='captcha']", "#captcha"],
    twoFaSelectors: ["#approvals_code", 'input[name="approvals_code"]'],
  },
};

const SUPPORTED: SocialService[] = Object.keys(FORM_SPECS) as SocialService[];

export class HeadlessFormAdapter extends BaseLoginAdapter {
  readonly name = "headless-form";
  readonly priority = 3;

  canHandle(service: SocialService): boolean {
    return SUPPORTED.includes(service);
  }

  async login(
    profile: BrowserProfile,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult> {
    const start = Date.now();
    const headless = options?.headless ?? true;
    const timeoutMs = options?.timeoutMs ?? 30_000;
    const spec = FORM_SPECS[service];
    if (!spec) {
      return {
        success: false,
        method: "headless_form",
        error: `No form spec for ${service}`,
        durationMs: Date.now() - start,
      };
    }

    let browser = null;
    try {
      browser = await this.launchBrowser(profile, headless);
      const context = await this.createStealthContext(browser);
      const page = await context.newPage();

      // Navigate to login page
      const loginUrl = SERVICE_LOGIN_URLS[service];
      await page.goto(loginUrl, {
        waitUntil: "domcontentloaded",
        timeout: 15_000,
      });

      // Human-like delay
      await new Promise((r) => setTimeout(r, getRandomDelay(500, 1500)));

      // Check for captcha before even trying
      const captchaDetected = await this.detectChallenge(page, spec.captchaSelectors);
      if (captchaDetected) {
        const screenshot = await this.takeScreenshot(page);
        options?.onInteractionRequired?.({
          type: "captcha",
          message: `Captcha rilevato su ${service}. Intervento manuale necessario.`,
          screenshotBase64: screenshot,
          remainingSeconds: Math.floor(timeoutMs / 1000),
        });
        await page.close();
        await context.close();
        return {
          success: false,
          method: "headless_form",
          error: "Captcha detected — user interaction required",
          requiresInteraction: true,
          screenshotBase64: screenshot,
          durationMs: Date.now() - start,
        };
      }

      // Fill username
      await this.fillField(page, spec.usernameSelector, ""); // TODO: credentials from secure store
      if (spec.nextButtonSelector) {
        await page.click(spec.nextButtonSelector);
        await new Promise((r) => setTimeout(r, getRandomDelay(1000, 2000)));
      }

      // Fill password
      await this.fillField(page, spec.passwordSelector, ""); // TODO: credentials from secure store

      // Submit
      await page.click(spec.submitSelector);

      // Wait for either login success or 2FA challenge
      const result = await Promise.race([
        this.waitForLogin(page, service, timeoutMs).then((ok) => ({
          type: "login" as const,
          ok,
        })),
        this.waitForChallenge(page, spec.twoFaSelectors, timeoutMs).then(
          (detected) => ({ type: "2fa" as const, ok: !detected }),
        ),
      ]);

      if (result.type === "2fa" && !result.ok) {
        const screenshot = await this.takeScreenshot(page);
        options?.onInteractionRequired?.({
          type: "2fa",
          message: `Verifica 2FA richiesta su ${service}.`,
          screenshotBase64: screenshot,
          remainingSeconds: 60,
        });
        await page.close();
        await context.close();
        return {
          success: false,
          method: "headless_form",
          error: "2FA verification required",
          requiresInteraction: true,
          screenshotBase64: screenshot,
          durationMs: Date.now() - start,
        };
      }

      await page.close();
      await context.close();

      return {
        success: result.ok,
        method: "headless_form",
        error: result.ok ? undefined : "Login form fill failed",
        durationMs: Date.now() - start,
      };
    } catch (err) {
      return {
        success: false,
        method: "headless_form",
        error: `Form fill login failed: ${err instanceof Error ? err.message : "unknown"}`,
        durationMs: Date.now() - start,
      };
    } finally {
      await browser?.close().catch(() => {});
    }
  }

  /** Fill a form field with human-like typing. */
  private async fillField(
    page: Page,
    selector: string,
    value: string,
  ): Promise<void> {
    await page.waitForSelector(selector, { timeout: 5000 });
    await page.click(selector);
    await new Promise((r) => setTimeout(r, getRandomDelay(100, 300)));
    // Type with random delays between keystrokes
    for (const char of value) {
      await page.keyboard.type(char, {
        delay: getRandomDelay(50, 150),
      });
    }
  }

  /** Detect if any challenge selector is present. */
  private async detectChallenge(
    page: Page,
    selectors: string[],
  ): Promise<boolean> {
    for (const sel of selectors) {
      try {
        const el = await page.$(sel);
        if (el) return true;
      } catch {
        continue;
      }
    }
    return false;
  }

  /** Wait for a challenge (captcha/2FA) to appear. */
  private async waitForChallenge(
    page: Page,
    selectors: string[],
    timeoutMs: number,
  ): Promise<boolean> {
    if (selectors.length === 0) return false;
    try {
      await page.waitForSelector(selectors.join(", "), {
        timeout: timeoutMs,
      });
      return true;
    } catch {
      return false;
    }
  }
}
