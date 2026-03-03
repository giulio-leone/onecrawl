/**
 * Two-Factor Authentication Challenge Detector
 *
 * Pure function that inspects page HTML for known LinkedIn 2FA patterns
 * and returns a structured TwoFactorChallenge, or null if no 2FA is detected.
 *
 * Fully deterministic — no side effects.
 */

import type { TwoFactorChallenge } from "../ports/twofa.port.js";

// =============================================================================
// Pattern sets for each 2FA method
// =============================================================================

const TOTP_PATTERNS = [
  /enter\s+(?:the\s+)?code\s+from\s+your\s+authenticator\s+app/i,
  /authenticator\s+app\s+verification/i,
  /two[- ]?step\s+verification.*authenticator/i,
  /verification\s+code.*authenticator/i,
];

const SMS_PATTERNS = [
  /enter\s+(?:the\s+)?code\s+we\s+sent\s+to\s+[\s\S]*?\*{2,}[\d\s()-]+/i,
  /we\s+(?:sent|texted)\s+(?:a\s+)?code\s+to\s+.*\d/i,
  /text\s+message.*verification\s+code/i,
  /sms\s+verification/i,
];

const EMAIL_PATTERNS = [
  /enter\s+(?:the\s+)?code\s+we\s+sent\s+to\s+[\s\S]*?[\w.*]+@[\w.]+/i,
  /we\s+(?:sent|emailed)\s+(?:a\s+)?code\s+to\s+.*@/i,
  /email\s+verification\s+code/i,
  /check\s+your\s+email\s+for\s+(?:a\s+)?verification\s+code/i,
];

// =============================================================================
// Hint extraction helpers
// =============================================================================

/** Extract the masked phone number hint shown to the user. */
function extractPhoneHint(html: string): string | undefined {
  const match = html.match(
    /(?:sent\s+to|code\s+to)\s+([\s\S]*?(\*{2,}[\d\s()-]+\d))/i,
  );
  return match?.[2]?.trim();
}

/** Extract the masked email hint shown to the user. */
function extractEmailHint(html: string): string | undefined {
  const match = html.match(
    /(?:sent\s+to|code\s+to)\s+([\s\S]*?([\w.*]+@[\w.*]+))/i,
  );
  return match?.[2]?.trim();
}

// =============================================================================
// Public API
// =============================================================================

/**
 * Inspect raw page content (HTML) for a 2FA challenge.
 *
 * @param pageContent Full or partial page HTML / text content
 * @returns Structured challenge descriptor, or `null` if no 2FA detected
 */
export function detectChallenge(
  pageContent: string,
): TwoFactorChallenge | null {
  // Check TOTP first (most common for power-users)
  for (const pattern of TOTP_PATTERNS) {
    if (pattern.test(pageContent)) {
      return { method: "totp" };
    }
  }

  // SMS
  for (const pattern of SMS_PATTERNS) {
    if (pattern.test(pageContent)) {
      return {
        method: "sms",
        hint: extractPhoneHint(pageContent),
      };
    }
  }

  // Email
  for (const pattern of EMAIL_PATTERNS) {
    if (pattern.test(pageContent)) {
      return {
        method: "email",
        hint: extractEmailHint(pageContent),
      };
    }
  }

  return null;
}
