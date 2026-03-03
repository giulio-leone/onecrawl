/**
 * OAuth Token Refresh Manager
 *
 * Periodically checks token expiry and refreshes before it lapses.
 * Uses an EventEmitter pattern for success/error notifications.
 */

import { EventEmitter } from "events";
import type { OAuthPort } from "../ports/oauth.port.js";
import type { OAuthTokenStore } from "./oauth-token-store.js";

const DEFAULT_CHECK_INTERVAL_MS = 60_000; // 1 minute
const EXPIRY_BUFFER_MS = 5 * 60_000; // Refresh if < 5 min remaining

export interface OAuthRefreshEvents {
  refresh_success: [{ expiresAt: number }];
  refresh_error: [Error];
}

export class OAuthRefreshManager extends EventEmitter<OAuthRefreshEvents> {
  private timer: ReturnType<typeof setInterval> | null = null;

  /**
   * Start the background refresh loop.
   * Checks periodically whether the stored token is near expiry and refreshes.
   */
  startRefreshLoop(
    adapter: OAuthPort,
    tokenStore: OAuthTokenStore,
    intervalMs = DEFAULT_CHECK_INTERVAL_MS,
  ): void {
    this.stopRefreshLoop();

    this.timer = setInterval(async () => {
      try {
        const tokens = await tokenStore.getTokens();
        if (!tokens?.refreshToken) return;

        const remaining = tokens.expiresAt - Date.now();
        if (remaining > EXPIRY_BUFFER_MS) return;

        const refreshed = await adapter.refreshToken(tokens.refreshToken);
        await tokenStore.saveTokens(refreshed);
        this.emit("refresh_success", { expiresAt: refreshed.expiresAt });
      } catch (err) {
        this.emit(
          "refresh_error",
          err instanceof Error ? err : new Error(String(err)),
        );
      }
    }, intervalMs);

    // Don't keep the process alive just for token refresh
    if (this.timer.unref) this.timer.unref();
  }

  /** Stop the background refresh loop. */
  stopRefreshLoop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }
}
