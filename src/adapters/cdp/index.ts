/**
 * CDP Adapters - Direct Chrome DevTools Protocol
 */

export {
  CDPClient,
  CDPPage,
  type CDPClientOptions,
  type CDPPageInfo,
} from "./client.js";
export { findChrome } from "./chrome-finder.js";
export {
  CDPScraperAdapter,
  createCDPScraperAdapter,
} from "./scraper.adapter.js";
