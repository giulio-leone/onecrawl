/**
 * TOTP (Time-based One-Time Password) implementation following RFC 6238.
 *
 * Uses only Node.js built-in `crypto` module.
 * - HMAC-SHA1 algorithm
 * - 30-second time step
 * - 6-digit output
 * - Accepts base32-encoded secrets
 */

import { createHmac } from "crypto";

// =============================================================================
// Base32 Decoding (RFC 4648)
// =============================================================================

const BASE32_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/** Decode a base32-encoded string to a Buffer. */
export function base32Decode(input: string): Buffer {
  const cleaned = input.replace(/[\s=]/g, "").toUpperCase();
  let bits = "";

  for (const char of cleaned) {
    const idx = BASE32_CHARS.indexOf(char);
    if (idx === -1) throw new Error(`Invalid base32 character: ${char}`);
    bits += idx.toString(2).padStart(5, "0");
  }

  const bytes: number[] = [];
  for (let i = 0; i + 8 <= bits.length; i += 8) {
    bytes.push(parseInt(bits.substring(i, i + 8), 2));
  }

  return Buffer.from(bytes);
}

// =============================================================================
// HMAC-SHA1
// =============================================================================

/** Compute HMAC-SHA1 of a big-endian 8-byte counter using the given key. */
export function hmacSha1(key: Buffer, counter: bigint): Buffer {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64BE(counter);
  return createHmac("sha1", key).update(buf).digest();
}

// =============================================================================
// TOTP Generation & Verification
// =============================================================================

export interface TOTPOptions {
  /** Time step in seconds (default: 30). */
  timeStep?: number;
  /** Number of output digits (default: 6). */
  digits?: number;
  /** Override current time in seconds since epoch (for testing). */
  time?: number;
}

/**
 * Generate a TOTP code from a base32-encoded secret.
 *
 * @param secret Base32-encoded shared secret
 * @param options TOTP parameters
 * @returns Zero-padded OTP string
 */
export function generateTOTP(secret: string, options: TOTPOptions = {}): string {
  const { timeStep = 30, digits = 6, time } = options;
  const key = base32Decode(secret);
  const now = time ?? Math.floor(Date.now() / 1000);
  const counter = BigInt(Math.floor(now / timeStep));

  const hash = hmacSha1(key, counter);

  // Dynamic truncation (RFC 4226 §5.4)
  const offset = hash[hash.length - 1]! & 0x0f;
  const binary =
    ((hash[offset]! & 0x7f) << 24) |
    ((hash[offset + 1]! & 0xff) << 16) |
    ((hash[offset + 2]! & 0xff) << 8) |
    (hash[offset + 3]! & 0xff);

  const otp = binary % 10 ** digits;
  return otp.toString().padStart(digits, "0");
}

/**
 * Verify a TOTP code against the current time with a tolerance window.
 *
 * @param secret  Base32-encoded shared secret
 * @param code    The code to verify
 * @param window  Number of time-steps to check in each direction (default: 1)
 * @param testTime Optional time override in seconds since epoch (for testing)
 * @returns `true` if the code matches any step within the window
 */
export function verifyTOTP(
  secret: string,
  code: string,
  window = 1,
  testTime?: number,
): boolean {
  const timeStep = 30;
  const now = testTime ?? Math.floor(Date.now() / 1000);
  const currentCounter = Math.floor(now / timeStep);

  for (let i = -window; i <= window; i++) {
    const stepTime = (currentCounter + i) * timeStep;
    const generated = generateTOTP(secret, { time: stepTime });
    if (generated === code) return true;
  }

  return false;
}
