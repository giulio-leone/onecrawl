/**
 * Cookie Manager - Import cookies from browsers and inject into requests
 * Enables access to authenticated content without browser automation.
 */

import { readFile } from "fs/promises";
import { homedir } from "os";
import { join } from "path";

/** Cookie structure */
export interface Cookie {
  name: string;
  value: string;
  domain: string;
  path: string;
  expires?: number;
  httpOnly?: boolean;
  secure?: boolean;
  sameSite?: "Strict" | "Lax" | "None";
}

/** Cookie store */
export interface CookieStore {
  getCookies(domain: string): Cookie[];
  setCookies(cookies: Cookie[]): void;
  clearCookies(domain?: string): void;
}

/**
 * In-memory cookie store
 */
export class MemoryCookieStore implements CookieStore {
  private cookies = new Map<string, Cookie[]>();

  getCookies(domain: string): Cookie[] {
    const result: Cookie[] = [];
    const cleanDomain = domain.replace(/^www\./, "");

    for (const [storedDomain, cookies] of this.cookies) {
      // Match domain and parent domains
      if (cleanDomain === storedDomain || cleanDomain.endsWith(`.${storedDomain}`)) {
        const now = Date.now() / 1000;
        for (const cookie of cookies) {
          // Skip expired cookies
          if (cookie.expires && cookie.expires < now) continue;
          result.push(cookie);
        }
      }
    }

    return result;
  }

  setCookies(cookies: Cookie[]): void {
    for (const cookie of cookies) {
      const domain = cookie.domain.replace(/^\./, "");
      const existing = this.cookies.get(domain) ?? [];

      // Replace existing cookie with same name
      const idx = existing.findIndex((c) => c.name === cookie.name);
      if (idx >= 0) {
        existing[idx] = cookie;
      } else {
        existing.push(cookie);
      }

      this.cookies.set(domain, existing);
    }
  }

  clearCookies(domain?: string): void {
    if (domain) {
      this.cookies.delete(domain.replace(/^www\./, ""));
    } else {
      this.cookies.clear();
    }
  }
}

/**
 * Import cookies from Chrome browser
 */
export async function importChromeCookies(
  profile = "Default",
): Promise<Cookie[]> {
  const cookiePaths = [
    // macOS
    join(
      homedir(),
      "Library/Application Support/Google/Chrome",
      profile,
      "Cookies",
    ),
    // Linux
    join(homedir(), ".config/google-chrome", profile, "Cookies"),
    // Windows
    join(
      process.env.LOCALAPPDATA ?? "",
      "Google/Chrome/User Data",
      profile,
      "Cookies",
    ),
  ];

  // Note: Chrome cookies are encrypted. This requires additional decryption.
  // For now, return empty - in production, use a library like `chrome-cookies-secure`
  console.warn(
    "Chrome cookie import requires decryption. Use chrome-cookies-secure or export manually.",
  );

  return [];
}

/**
 * Import cookies from Firefox browser
 */
export async function importFirefoxCookies(
  profile?: string,
): Promise<Cookie[]> {
  const firefoxPath =
    process.platform === "darwin"
      ? join(homedir(), "Library/Application Support/Firefox/Profiles")
      : process.platform === "win32"
        ? join(process.env.APPDATA ?? "", "Mozilla/Firefox/Profiles")
        : join(homedir(), ".mozilla/firefox");

  // Note: Firefox cookies are in SQLite. Would need better-sqlite3.
  console.warn(
    "Firefox cookie import requires SQLite. Use cookies.txt export instead.",
  );

  return [];
}

/**
 * Import cookies from Netscape cookies.txt format
 * This is the most portable format, exported by many browser extensions.
 */
export async function importCookiesTxt(filePath: string): Promise<Cookie[]> {
  const content = await readFile(filePath, "utf-8");
  const cookies: Cookie[] = [];

  for (const line of content.split("\n")) {
    // Skip comments and empty lines
    if (line.startsWith("#") || !line.trim()) continue;

    const parts = line.split("\t");
    if (parts.length < 7) continue;

    const [domain, , path, secure, expires, name, value] = parts;

    cookies.push({
      name: name!,
      value: value!,
      domain: domain!,
      path: path!,
      expires: parseInt(expires!, 10) || undefined,
      secure: secure === "TRUE",
    });
  }

  return cookies;
}

/**
 * Export cookies to Netscape cookies.txt format
 */
export function exportCookiesTxt(cookies: Cookie[]): string {
  const lines = ["# Netscape HTTP Cookie File"];

  for (const cookie of cookies) {
    const line = [
      cookie.domain,
      cookie.domain.startsWith(".") ? "TRUE" : "FALSE",
      cookie.path,
      cookie.secure ? "TRUE" : "FALSE",
      cookie.expires ?? 0,
      cookie.name,
      cookie.value,
    ].join("\t");

    lines.push(line);
  }

  return lines.join("\n");
}

/**
 * Convert cookies to HTTP Cookie header
 */
export function cookiesToHeader(cookies: Cookie[]): string {
  return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
}

/**
 * Parse Set-Cookie header into Cookie objects
 */
export function parseSetCookie(header: string, domain: string): Cookie {
  const parts = header.split(";").map((p) => p.trim());
  const [nameValue, ...attrs] = parts;
  const [name, value] = nameValue!.split("=");

  const cookie: Cookie = {
    name: name!,
    value: value ?? "",
    domain,
    path: "/",
  };

  for (const attr of attrs) {
    const [key, val] = attr.split("=");
    const lowerKey = key!.toLowerCase();

    if (lowerKey === "domain") cookie.domain = val ?? domain;
    else if (lowerKey === "path") cookie.path = val ?? "/";
    else if (lowerKey === "expires") {
      cookie.expires = Math.floor(new Date(val!).getTime() / 1000);
    } else if (lowerKey === "max-age") {
      cookie.expires = Math.floor(Date.now() / 1000) + parseInt(val!, 10);
    } else if (lowerKey === "secure") cookie.secure = true;
    else if (lowerKey === "httponly") cookie.httpOnly = true;
    else if (lowerKey === "samesite") {
      cookie.sameSite = val as Cookie["sameSite"];
    }
  }

  return cookie;
}
