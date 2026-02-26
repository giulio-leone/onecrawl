/**
 * OneCrawl Auth - React Native Compatible
 * All cookie functions are now platform-agnostic via StoragePort.
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
  StaticProxyPool,
  RotatingProxyService,
  proxyToUrl,
  parseProxyUrl,
  type ProxyConfig,
  type ProxyProvider,
} from "./proxy.js";
