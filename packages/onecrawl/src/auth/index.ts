/**
 * OneCrawl Auth - Cookie and proxy management
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
