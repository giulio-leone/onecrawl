/**
 * Profile Manager Use-Case
 *
 * Manages browser profiles, orchestrates the login chain,
 * and handles background session verification.
 *
 * Login chain fallback order:
 *   1. PasskeyAdapter (headless, instant)
 *   2. PopupOAuthAdapter (mini popup window, 60-120s)
 *   3. HeadlessFormAdapter (headless form fill, screenshot on captcha)
 *   4. HeadedFullAdapter (full Chrome UI, last resort)
 */

import { mkdir, readdir, readFile, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { homedir } from "node:os";
import { randomUUID } from "node:crypto";
import type {
  BrowserProfile,
  LoginResult,
  ProfileManagerConfig,
  ServiceSession,
  SessionInfo,
  SocialService,
} from "../domain/browser-profile.js";
import type { LoginPort, LoginOptions } from "../ports/login.port.js";

const DEFAULT_CONFIG: ProfileManagerConfig = {
  profilesDir: join(homedir(), ".onecrawl", "profiles"),
  verifyIntervalMs: 4 * 60 * 60 * 1000, // 4 hours
  defaultHeadless: true,
  cdpPortRange: [9400, 9499],
};

const PROFILE_META_FILE = "profile.json";

export class ProfileManager {
  private config: ProfileManagerConfig;
  private adapters: LoginPort[] = [];
  private verifyTimer: ReturnType<typeof setInterval> | null = null;
  private profiles = new Map<string, BrowserProfile>();

  constructor(
    adapters: LoginPort[],
    config: Partial<ProfileManagerConfig> = {},
  ) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    // Sort adapters by priority (lower = tried first)
    this.adapters = [...adapters].sort((a, b) => a.priority - b.priority);
  }

  // ── Profile CRUD ──────────────────────────────────────────────────────

  /** Initialize: load all existing profiles from disk. */
  async init(): Promise<void> {
    await mkdir(this.config.profilesDir, { recursive: true });
    const entries = await readdir(this.config.profilesDir, {
      withFileTypes: true,
    });
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      try {
        const metaPath = join(
          this.config.profilesDir,
          entry.name,
          PROFILE_META_FILE,
        );
        const raw = await readFile(metaPath, "utf-8");
        const profile = JSON.parse(raw) as BrowserProfile;
        this.profiles.set(profile.id, profile);
      } catch {
        // Skip invalid profile directories
      }
    }
  }

  /** Create a new browser profile. */
  async createProfile(
    name: string,
    services: SocialService[] = [],
    headless?: boolean,
  ): Promise<BrowserProfile> {
    const id = randomUUID();
    const userDataDir = join(this.config.profilesDir, id);
    await mkdir(userDataDir, { recursive: true });

    const profile: BrowserProfile = {
      id,
      name,
      userDataDir,
      services: services.map(
        (service): ServiceSession => ({
          service,
          status: "never_logged",
          lastVerified: null,
          expiresAt: null,
          loginMethod: null,
        }),
      ),
      headless: headless ?? this.config.defaultHeadless,
      createdAt: new Date().toISOString(),
      lastVerified: null,
    };

    await this.saveProfile(profile);
    this.profiles.set(id, profile);
    return profile;
  }

  /** Get a profile by ID. */
  getProfile(id: string): BrowserProfile | undefined {
    return this.profiles.get(id);
  }

  /** List all profiles. */
  listProfiles(): BrowserProfile[] {
    return Array.from(this.profiles.values());
  }

  /** Delete a profile and its user-data-dir. */
  async deleteProfile(id: string): Promise<void> {
    const profile = this.profiles.get(id);
    if (!profile) return;
    await rm(profile.userDataDir, { recursive: true, force: true });
    this.profiles.delete(id);
  }

  /** Update profile fields and persist. */
  async updateProfile(
    id: string,
    updates: Partial<Pick<BrowserProfile, "name" | "headless" | "services">>,
  ): Promise<BrowserProfile | null> {
    const profile = this.profiles.get(id);
    if (!profile) return null;
    Object.assign(profile, updates);
    await this.saveProfile(profile);
    return profile;
  }

  /** Add a service to a profile. */
  async addService(id: string, service: SocialService): Promise<void> {
    const profile = this.profiles.get(id);
    if (!profile) return;
    if (profile.services.some((s) => s.service === service)) return;
    profile.services.push({
      service,
      status: "never_logged",
      lastVerified: null,
      expiresAt: null,
      loginMethod: null,
    });
    await this.saveProfile(profile);
  }

  // ── Login Chain ───────────────────────────────────────────────────────

  /**
   * Attempt to log in to a service using the adapter chain.
   * Tries each adapter in priority order until one succeeds.
   */
  async login(
    profileId: string,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult> {
    const profile = this.profiles.get(profileId);
    if (!profile)
      return {
        success: false,
        method: null,
        error: `Profile ${profileId} not found`,
        durationMs: 0,
      };

    const start = Date.now();
    const errors: string[] = [];

    for (const adapter of this.adapters) {
      if (!adapter.canHandle(service)) continue;

      try {
        const result = await adapter.login(profile, service, {
          ...options,
          headless: options?.headless ?? profile.headless,
        });

        if (result.success) {
          // Update session state
          this.updateServiceSession(profile, service, {
            status: "active",
            lastVerified: new Date().toISOString(),
            loginMethod: result.method,
          });
          profile.lastVerified = new Date().toISOString();
          await this.saveProfile(profile);
          return { ...result, durationMs: Date.now() - start };
        }

        errors.push(`[${adapter.name}] ${result.error ?? "Failed"}`);

        // If adapter says it needs user interaction and we have more adapters, continue
        if (result.requiresInteraction && options?.onInteractionRequired) {
          // Adapter already invoked the callback
        }
      } catch (err) {
        errors.push(
          `[${adapter.name}] ${err instanceof Error ? err.message : "Unknown error"}`,
        );
      }
    }

    return {
      success: false,
      method: null,
      error: `All login adapters failed:\n${errors.join("\n")}`,
      durationMs: Date.now() - start,
    };
  }

  // ── Session Verification ──────────────────────────────────────────────

  /** Verify a single service session. */
  async verifySession(
    profileId: string,
    service: SocialService,
  ): Promise<SessionInfo> {
    const profile = this.profiles.get(profileId);
    if (!profile)
      return { valid: false, service, expiresAt: null, detail: "Profile not found" };

    // Use the first adapter that can handle the service
    for (const adapter of this.adapters) {
      if (!adapter.canHandle(service)) continue;
      try {
        const info = await adapter.verifySession(profile, service);
        this.updateServiceSession(profile, service, {
          status: info.valid ? "active" : "expired",
          lastVerified: new Date().toISOString(),
          expiresAt: info.expiresAt,
        });
        await this.saveProfile(profile);
        return info;
      } catch {
        continue;
      }
    }

    return { valid: false, service, expiresAt: null, detail: "No adapter available" };
  }

  /** Verify all sessions across all profiles. */
  async verifyAllSessions(): Promise<Map<string, SessionInfo[]>> {
    const results = new Map<string, SessionInfo[]>();
    for (const profile of this.profiles.values()) {
      const profileResults: SessionInfo[] = [];
      for (const session of profile.services) {
        if (session.status === "never_logged") continue;
        const info = await this.verifySession(profile.id, session.service);
        profileResults.push(info);
      }
      if (profileResults.length > 0) results.set(profile.id, profileResults);
    }
    return results;
  }

  /** Start background session verification loop. */
  startVerificationLoop(
    onExpired?: (profileId: string, service: SocialService) => void,
  ): void {
    if (this.verifyTimer) return;
    this.verifyTimer = setInterval(async () => {
      const results = await this.verifyAllSessions();
      for (const [profileId, infos] of results) {
        for (const info of infos) {
          if (!info.valid && onExpired) {
            onExpired(profileId, info.service);
          }
        }
      }
    }, this.config.verifyIntervalMs);
  }

  /** Stop the background verification loop. */
  stopVerificationLoop(): void {
    if (this.verifyTimer) {
      clearInterval(this.verifyTimer);
      this.verifyTimer = null;
    }
  }

  /** Close all adapters. */
  async close(): Promise<void> {
    this.stopVerificationLoop();
    await Promise.all(this.adapters.map((a) => a.close()));
  }

  // ── Helpers ───────────────────────────────────────────────────────────

  private async saveProfile(profile: BrowserProfile): Promise<void> {
    const metaPath = join(profile.userDataDir, PROFILE_META_FILE);
    await writeFile(metaPath, JSON.stringify(profile, null, 2), "utf-8");
  }

  private updateServiceSession(
    profile: BrowserProfile,
    service: SocialService,
    updates: Partial<ServiceSession>,
  ): void {
    const idx = profile.services.findIndex((s) => s.service === service);
    if (idx >= 0) {
      Object.assign(profile.services[idx], updates);
    } else {
      profile.services.push({
        service,
        status: "never_logged",
        lastVerified: null,
        expiresAt: null,
        loginMethod: null,
        ...updates,
      });
    }
  }
}
