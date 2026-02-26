/**
 * Cookie Manager - Import cookies and inject into requests
 * Platform-agnostic: no direct fs/os/path imports.
 */

import type { StoragePort } from "../ports/storage.port.js";

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
      if (
        cleanDomain === storedDomain ||
        cleanDomain.endsWith(`.${storedDomain}`)
      ) {
        const now = Date.now() / 1000;
        for (const cookie of cookies) {
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
 * Parse Netscape cookies.txt content into Cookie objects.
 * Platform-agnostic: operates on string content only.
 */
export function parseCookiesTxt(content: string): Cookie[] {
  const cookies: Cookie[] = [];

  for (const line of content.split("\n")) {
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
 * Import cookies from a raw cookies.txt string.
 * Alias for parseCookiesTxt for backward compatibility.
 */
export function importCookiesTxt(content: string): Cookie[] {
  return parseCookiesTxt(content);
}

/**
 * Import cookies from StoragePort by key.
 * Reads a cookies.txt-formatted string from storage and parses it.
 */
export async function importCookiesTxtFromStorage(
  storage: StoragePort,
  key: string,
): Promise<Cookie[]> {
  const content = await storage.get(key);
  if (!content) return [];
  return parseCookiesTxt(content);
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
