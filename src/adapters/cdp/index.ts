/**
 * CDP Adapters - Direct Chrome DevTools Protocol
 */

export { CDPClient, CDPPage, type CDPClientOptions } from "./client.js";
export {
  CDPScraperAdapter,
  createCDPScraperAdapter,
} from "./scraper.adapter.js";
