/**
 * SMS Two-Factor Authentication Adapter
 *
 * Handles the human-in-the-loop flow for SMS-based 2FA challenges.
 * Designed for both CLI (stdin prompt) and MCP (ask_user) integration
 * via an injectable `onCodeRequired` callback.
 */

import type {
  TwoFactorChallenge,
  TwoFactorMethod,
  TwoFactorPort,
} from "../ports/twofa.port.js";
import { detectChallenge } from "./twofa-detector.js";

export interface Sms2faAdapterOptions {
  /** Callback invoked when a code is needed from the user. */
  onCodeRequired: () => Promise<string>;
  /** Returns the current page HTML for challenge detection. */
  getPageContent: () => Promise<string>;
  /** Submits the verification code via the page UI. Returns success. */
  submitCodeToPage: (code: string) => Promise<boolean>;
}

export class Sms2faAdapter implements TwoFactorPort {
  private readonly onCodeRequired: () => Promise<string>;
  private readonly getPageContent: () => Promise<string>;
  private readonly submitCodeToPage: (code: string) => Promise<boolean>;

  constructor(options: Sms2faAdapterOptions) {
    this.onCodeRequired = options.onCodeRequired;
    this.getPageContent = options.getPageContent;
    this.submitCodeToPage = options.submitCodeToPage;
  }

  /** Detect if the current page shows an SMS 2FA challenge. */
  async detect(): Promise<TwoFactorChallenge | null> {
    const html = await this.getPageContent();
    const challenge = detectChallenge(html);
    if (challenge && challenge.method === "sms") {
      return challenge;
    }
    return null;
  }

  /** Request the SMS code from the user via the injected callback. */
  async getCode(_method: TwoFactorMethod): Promise<string> {
    return this.onCodeRequired();
  }

  /** Submit the user-provided code to the verification form. */
  async submitCode(code: string, _challengeId?: string): Promise<boolean> {
    return this.submitCodeToPage(code);
  }
}
