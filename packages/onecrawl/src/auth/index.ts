/**
 * OneCrawl Auth - Cookie, proxy, passkey, and WebAuthn management
 */

export {
  MemoryCookieStore,
  parseCookiesTxt,
  importCookiesTxt,
  importCookiesTxtFromStorage,
  exportCookiesTxt,
  cookiesToHeader,
  parseSetCookie,
  type Cookie,
  type CookieStore,
} from "./cookies.js";

export {
  AuthCascade,
  type AuthMethod,
  type AuthResult,
  type AuthCascadeOptions,
} from "./auth-cascade.js";

export {
  StaticProxyPool,
  RotatingProxyService,
  proxyToUrl,
  parseProxyUrl,
  type ProxyConfig,
  type ProxyProvider,
} from "./proxy.js";

export {
  WebAuthnManager,
  type CDPSession,
  type AuthenticatorOptions,
  type WebAuthnCredential,
} from "./webauthn-manager.js";

export {
  PasskeyStore,
  type PasskeyStoreData,
} from "./passkey-store.js";
