/**
 * Browser Profile domain types.
 * Each profile = an isolated Chrome user-data-dir with its own cookies/sessions.
 */

/** Known social services that require browser-based authentication. */
export type SocialService =
  | "x"
  | "linkedin"
  | "instagram"
  | "facebook"
  | "threads"
  | "tiktok"
  | "youtube"
  | "pinterest"
  | "reddit";

/** Current state of a service login within a profile. */
export type SessionStatus = "active" | "expired" | "never_logged" | "verifying";

/** A single service session attached to a profile. */
export interface ServiceSession {
  service: SocialService;
  status: SessionStatus;
  lastVerified: string | null;
  expiresAt: string | null;
  loginMethod: "passkey" | "popup" | "headless_form" | "headed" | null;
}

/** A named browser profile backed by a Chrome user-data-dir. */
export interface BrowserProfile {
  id: string;
  name: string;
  /** Absolute path to the Chrome user-data-dir. */
  userDataDir: string;
  /** Service sessions tracked for this profile. */
  services: ServiceSession[];
  /** Whether to run headless by default. */
  headless: boolean;
  /** When the profile was created (ISO). */
  createdAt: string;
  /** When any session was last verified (ISO). */
  lastVerified: string | null;
}

/** Result of a login attempt. */
export interface LoginResult {
  success: boolean;
  /** Which adapter strategy succeeded. */
  method: "passkey" | "popup" | "headless_form" | "headed" | null;
  /** Error message if failed. */
  error?: string;
  /** True if the adapter needs user interaction (e.g. 2FA prompt). */
  requiresInteraction?: boolean;
  /** Screenshot buffer (png) when user action is needed (captcha, 2FA). */
  screenshotBase64?: string;
  /** Duration of the login attempt in ms. */
  durationMs: number;
}

/** Lightweight session info returned by verification check. */
export interface SessionInfo {
  valid: boolean;
  service: SocialService;
  expiresAt: string | null;
  /** Human-readable detail (e.g. "Cookie will expire in 2d"). */
  detail?: string;
}

/** Config for the profile manager. */
export interface ProfileManagerConfig {
  /** Base directory for all profiles (default: ~/.onecrawl/profiles). */
  profilesDir: string;
  /** How often to auto-verify sessions in ms (default: 4h). */
  verifyIntervalMs: number;
  /** Default headless mode for new profiles. */
  defaultHeadless: boolean;
  /** CDP port to use when launching Chrome instances. */
  cdpPortRange: [number, number];
}
