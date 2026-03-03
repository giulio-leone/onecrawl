/**
 * WebAuthn/FIDO2 Virtual Authenticator Manager
 * Uses Chrome DevTools Protocol to enable passkey authentication in browser contexts.
 * Compatible with both Playwright CDPSession and custom CDPPage.
 */

/** Minimal CDP session interface for WebAuthn domain commands. */
export interface CDPSession {
  send(method: string, params?: Record<string, unknown>): Promise<unknown>;
}

/** Options for creating a virtual authenticator. */
export interface AuthenticatorOptions {
  protocol?: "ctap2";
  ctap2Version?: "ctap2_0" | "ctap2_1";
  transport?: "usb" | "nfc" | "ble" | "cable" | "internal";
  hasResidentKey?: boolean;
  hasUserVerification?: boolean;
  isUserVerified?: boolean;
  automaticPresenceSimulation?: boolean;
}

/** WebAuthn credential stored in a virtual authenticator. */
export interface WebAuthnCredential {
  credentialId: string;
  isResidentCredential: boolean;
  rpId: string;
  privateKey: string;
  userHandle: string;
  signCount: number;
  largeBlob?: string;
}

/** Default authenticator options optimized for passkey flows. */
const PASSKEY_DEFAULTS: Required<AuthenticatorOptions> = {
  protocol: "ctap2",
  ctap2Version: "ctap2_1",
  transport: "internal",
  hasResidentKey: true,
  hasUserVerification: true,
  isUserVerified: true,
  automaticPresenceSimulation: true,
};

/**
 * Manages a virtual WebAuthn authenticator via CDP.
 * Wraps the WebAuthn domain to create/manage authenticators and credentials.
 */
export class WebAuthnManager {
  private _authenticatorId: string | null = null;

  constructor(private readonly cdp: CDPSession) {}

  /** Enable the WebAuthn environment (disables browser UI prompts). */
  async enable(): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.enable", { enableUI: false });
    } catch (err) {
      throw new Error(
        `WebAuthn.enable failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Disable the WebAuthn environment. */
  async disable(): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.disable");
    } catch (err) {
      throw new Error(
        `WebAuthn.disable failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /**
   * Add a virtual authenticator.
   * @returns The authenticator ID assigned by the browser.
   */
  async addAuthenticator(options?: AuthenticatorOptions): Promise<string> {
    const opts = { ...PASSKEY_DEFAULTS, ...options };

    try {
      const result = (await this.cdp.send("WebAuthn.addVirtualAuthenticator", {
        options: opts,
      })) as { authenticatorId: string };
      return result.authenticatorId;
    } catch (err) {
      throw new Error(
        `WebAuthn.addVirtualAuthenticator failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Remove a virtual authenticator by ID. */
  async removeAuthenticator(authenticatorId: string): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.removeVirtualAuthenticator", {
        authenticatorId,
      });
    } catch (err) {
      throw new Error(
        `WebAuthn.removeVirtualAuthenticator failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Add a credential to the specified authenticator. */
  async addCredential(
    authenticatorId: string,
    credential: WebAuthnCredential,
  ): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.addCredential", {
        authenticatorId,
        credential,
      });
    } catch (err) {
      throw new Error(
        `WebAuthn.addCredential failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Retrieve all credentials from the specified authenticator. */
  async getCredentials(
    authenticatorId: string,
  ): Promise<WebAuthnCredential[]> {
    try {
      const result = (await this.cdp.send("WebAuthn.getCredentials", {
        authenticatorId,
      })) as { credentials: WebAuthnCredential[] };
      return result.credentials;
    } catch (err) {
      throw new Error(
        `WebAuthn.getCredentials failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Remove a single credential from the specified authenticator. */
  async removeCredential(
    authenticatorId: string,
    credentialId: string,
  ): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.removeCredential", {
        authenticatorId,
        credentialId,
      });
    } catch (err) {
      throw new Error(
        `WebAuthn.removeCredential failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Clear all credentials from the specified authenticator. */
  async clearCredentials(authenticatorId: string): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.clearCredentials", { authenticatorId });
    } catch (err) {
      throw new Error(
        `WebAuthn.clearCredentials failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /** Set user-verified state on the specified authenticator. */
  async setUserVerified(
    authenticatorId: string,
    verified: boolean,
  ): Promise<void> {
    try {
      await this.cdp.send("WebAuthn.setUserVerified", {
        authenticatorId,
        isUserVerified: verified,
      });
    } catch (err) {
      throw new Error(
        `WebAuthn.setUserVerified failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  /**
   * Convenience: enable WebAuthn and create a default passkey authenticator.
   * Stores the authenticator ID for use by inject/extract helpers.
   */
  async setupForPasskeys(options?: AuthenticatorOptions): Promise<string> {
    await this.enable();
    this._authenticatorId = await this.addAuthenticator(options);
    return this._authenticatorId;
  }

  /**
   * Inject credentials into the default authenticator.
   * Calls setupForPasskeys automatically if no authenticator exists.
   */
  async injectCredentials(credentials: WebAuthnCredential[]): Promise<void> {
    if (!this._authenticatorId) {
      await this.setupForPasskeys();
    }

    for (const credential of credentials) {
      await this.addCredential(this._authenticatorId!, credential);
    }
  }

  /** Extract all credentials from the default authenticator. */
  async extractCredentials(): Promise<WebAuthnCredential[]> {
    if (!this._authenticatorId) {
      throw new Error(
        "No default authenticator — call setupForPasskeys() first",
      );
    }
    return this.getCredentials(this._authenticatorId);
  }

  /** Remove the default authenticator and disable WebAuthn. */
  async teardown(): Promise<void> {
    if (this._authenticatorId) {
      await this.removeAuthenticator(this._authenticatorId);
      this._authenticatorId = null;
    }
    await this.disable();
  }
}
