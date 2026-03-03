/**
 * Two-Factor Authentication Port.
 */

export type TwoFactorMethod = "totp" | "sms" | "email";

export interface TwoFactorChallenge {
  method: TwoFactorMethod;
  /** For SMS/email: partial phone/email shown by the service */
  hint?: string;
  /** Any service-specific challenge ID */
  challengeId?: string;
}

export interface TwoFactorPort {
  /** Detect if a 2FA challenge is present and what type. */
  detect(): Promise<TwoFactorChallenge | null>;
  /** Generate or retrieve a code for the given method. */
  getCode(method: TwoFactorMethod): Promise<string>;
  /** Submit a 2FA code. */
  submitCode(code: string, challengeId?: string): Promise<boolean>;
}
