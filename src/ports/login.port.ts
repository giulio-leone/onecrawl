/**
 * Login Port — hexagonal contract for browser-based authentication adapters.
 *
 * Each adapter implements a different login strategy:
 *   PasskeyAdapter → PopupOAuthAdapter → HeadlessFormAdapter → HeadedFullAdapter
 *
 * The ProfileManager use-case tries them in order until one succeeds.
 */

import type {
  BrowserProfile,
  LoginResult,
  SessionInfo,
  SocialService,
} from "../domain/browser-profile.js";

/**
 * Contract that every login strategy adapter must implement.
 */
export interface LoginPort {
  /** Human-readable adapter name (e.g. "passkey", "popup-oauth"). */
  readonly name: string;

  /** Priority (lower = tried first). Used for sorting the chain. */
  readonly priority: number;

  /**
   * Whether this adapter can handle the given service.
   * Some adapters may not support certain services (e.g. passkey only for X).
   */
  canHandle(service: SocialService): boolean;

  /**
   * Attempt to log in to a service using this strategy.
   * @param profile The browser profile to authenticate
   * @param service The social service to log into
   * @param options Extra options (timeout, onScreenshot callback, etc.)
   * @returns LoginResult — success or failure with details
   */
  login(
    profile: BrowserProfile,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult>;

  /**
   * Verify whether the current session for a service is still valid.
   * Should be a quick check (e.g. load profile page, check for login wall).
   */
  verifySession(
    profile: BrowserProfile,
    service: SocialService,
  ): Promise<SessionInfo>;

  /** Clean up any resources (browser instances, etc). */
  close(): Promise<void>;
}

/** Options passed to login adapters. */
export interface LoginOptions {
  /** Max time to wait for login completion in ms (default: 120_000). */
  timeoutMs?: number;
  /** Callback invoked when user interaction is needed (screenshot, prompt). */
  onInteractionRequired?: (data: InteractionData) => void;
  /** Abort signal for cancellation. */
  signal?: AbortSignal;
  /** Headless override (adapter may ignore if not applicable). */
  headless?: boolean;
}

/** Data sent to the user when interaction is needed. */
export interface InteractionData {
  type: "captcha" | "2fa" | "oauth_prompt" | "manual_login";
  message: string;
  /** Base64-encoded screenshot of the current state. */
  screenshotBase64?: string;
  /** Seconds remaining before timeout. */
  remainingSeconds: number;
}

/** Service URL mappings for login and verification pages. */
export const SERVICE_LOGIN_URLS: Record<SocialService, string> = {
  x: "https://x.com/i/flow/login",
  linkedin: "https://www.linkedin.com/login",
  instagram: "https://www.instagram.com/accounts/login/",
  facebook: "https://www.facebook.com/login",
  threads: "https://www.threads.net/login",
  tiktok: "https://www.tiktok.com/login",
  youtube: "https://accounts.google.com/ServiceLogin?service=youtube",
  pinterest: "https://www.pinterest.com/login/",
  reddit: "https://www.reddit.com/login/",
};

/** URLs used to verify if a session is still active. */
export const SERVICE_VERIFY_URLS: Record<SocialService, string> = {
  x: "https://x.com/home",
  linkedin: "https://www.linkedin.com/feed/",
  instagram: "https://www.instagram.com/",
  facebook: "https://www.facebook.com/",
  threads: "https://www.threads.net/",
  tiktok: "https://www.tiktok.com/foryou",
  youtube: "https://www.youtube.com/",
  pinterest: "https://www.pinterest.com/",
  reddit: "https://www.reddit.com/",
};

/** Selectors that indicate a successful login (service-specific). */
export const SERVICE_LOGGED_IN_SELECTORS: Record<SocialService, string[]> = {
  x: ['[data-testid="SideNav_AccountSwitcher_Button"]', '[aria-label="Profile"]'],
  linkedin: ['[data-control-name="identity_welcome_message"]', ".feed-identity-module"],
  instagram: ['[aria-label="Home"]', 'a[href="/direct/inbox/"]'],
  facebook: ['[aria-label="Facebook"]', '[data-pagelet="ProfileTilesFeed"]'],
  threads: ['[aria-label="Home"]', '[aria-label="New thread"]'],
  tiktok: ['[data-e2e="profile-icon"]'],
  youtube: ["#avatar-btn", "ytd-topbar-menu-button-renderer"],
  pinterest: ['[data-test-id="header-profile"]'],
  reddit: ['[data-testid="reddit-avatar"]', "#USER_DROPDOWN_ID"],
};
