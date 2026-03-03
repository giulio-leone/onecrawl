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

export {
  base32Decode,
  hmacSha1,
  generateTOTP,
  verifyTOTP,
  type TOTPOptions,
} from "./totp-adapter.js";

export {
  TotpSecretStore,
  type TotpSecretStoreOptions,
} from "./totp-secret-store.js";

export {
  Sms2faAdapter,
  type Sms2faAdapterOptions,
} from "./sms-2fa-adapter.js";

export { detectChallenge } from "./twofa-detector.js";

export {
  generateCodeVerifier,
  generateCodeChallenge,
  generateState,
} from "./oauth-pkce.js";

export {
  OAuthTokenStore,
  type OAuthTokenStoreOptions,
} from "./oauth-token-store.js";

export {
  OAuthRefreshManager,
  type OAuthRefreshEvents,
} from "./oauth-refresh.js";
