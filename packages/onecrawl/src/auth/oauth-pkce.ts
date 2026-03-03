/**
 * PKCE (Proof Key for Code Exchange) utilities for OAuth 2.1.
 *
 * Uses S256 code challenge method per RFC 7636.
 * Only Node.js built-in `crypto` module — zero dependencies.
 */

import { randomBytes, createHash } from "crypto";

const VERIFIER_CHARSET =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
const VERIFIER_MIN = 43;
const VERIFIER_MAX = 128;

/**
 * Generate a cryptographically random code verifier (43-128 URL-safe chars).
 */
export function generateCodeVerifier(length = 64): string {
  if (length < VERIFIER_MIN || length > VERIFIER_MAX) {
    throw new RangeError(
      `Code verifier length must be ${VERIFIER_MIN}-${VERIFIER_MAX}, got ${length}`,
    );
  }

  const bytes = randomBytes(length);
  let verifier = "";
  for (let i = 0; i < length; i++) {
    verifier += VERIFIER_CHARSET[bytes[i]! % VERIFIER_CHARSET.length];
  }
  return verifier;
}

/**
 * Derive the S256 code challenge from a code verifier.
 *
 * challenge = BASE64URL(SHA256(verifier))
 */
export function generateCodeChallenge(verifier: string): string {
  return createHash("sha256")
    .update(verifier, "ascii")
    .digest("base64url");
}

/**
 * Generate a random state parameter for CSRF protection.
 */
export function generateState(): string {
  return randomBytes(32).toString("base64url");
}
