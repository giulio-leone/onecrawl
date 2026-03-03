/**
 * LinkedIn OAuth 2.1 Adapter — implements OAuthPort for LinkedIn.
 *
 * Uses native `fetch` only (no external HTTP dependencies).
 *
 * Endpoints:
 *   Authorize → https://www.linkedin.com/oauth/v2/authorization
 *   Token    → https://www.linkedin.com/oauth/v2/accessToken
 *   Revoke   → https://www.linkedin.com/oauth/v2/revoke
 */

import type {
  OAuthPort,
  OAuthConfig,
  OAuthTokens,
} from "../../ports/oauth.port.js";

const DEFAULT_SCOPES = ["openid", "profile", "email"];

export class LinkedInOAuthAdapter implements OAuthPort {
  private readonly config: OAuthConfig;

  constructor(config: Partial<OAuthConfig> & Pick<OAuthConfig, "clientId" | "redirectUri">) {
    this.config = {
      authorizeEndpoint: "https://www.linkedin.com/oauth/v2/authorization",
      tokenEndpoint: "https://www.linkedin.com/oauth/v2/accessToken",
      revokeEndpoint: "https://www.linkedin.com/oauth/v2/revoke",
      scopes: DEFAULT_SCOPES,
      ...config,
    };
  }

  getAuthorizationUrl(state: string, codeChallenge: string): string {
    const params = new URLSearchParams({
      response_type: "code",
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      state,
      scope: this.config.scopes.join(" "),
      code_challenge: codeChallenge,
      code_challenge_method: "S256",
    });

    return `${this.config.authorizeEndpoint}?${params.toString()}`;
  }

  async exchangeCode(
    code: string,
    codeVerifier: string,
  ): Promise<OAuthTokens> {
    const body = new URLSearchParams({
      grant_type: "authorization_code",
      code,
      redirect_uri: this.config.redirectUri,
      client_id: this.config.clientId,
      code_verifier: codeVerifier,
    });

    if (this.config.clientSecret) {
      body.set("client_secret", this.config.clientSecret);
    }

    const res = await fetch(this.config.tokenEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body.toString(),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Token exchange failed (${res.status}): ${text}`);
    }

    const data = (await res.json()) as {
      access_token: string;
      refresh_token?: string;
      expires_in: number;
      token_type: string;
      scope?: string;
    };

    return {
      accessToken: data.access_token,
      refreshToken: data.refresh_token,
      expiresAt: Date.now() + data.expires_in * 1_000,
      tokenType: data.token_type,
      scope: data.scope,
    };
  }

  async refreshToken(refreshToken: string): Promise<OAuthTokens> {
    const body = new URLSearchParams({
      grant_type: "refresh_token",
      refresh_token: refreshToken,
      client_id: this.config.clientId,
    });

    if (this.config.clientSecret) {
      body.set("client_secret", this.config.clientSecret);
    }

    const res = await fetch(this.config.tokenEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body.toString(),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Token refresh failed (${res.status}): ${text}`);
    }

    const data = (await res.json()) as {
      access_token: string;
      refresh_token?: string;
      expires_in: number;
      token_type: string;
      scope?: string;
    };

    return {
      accessToken: data.access_token,
      refreshToken: data.refresh_token ?? refreshToken,
      expiresAt: Date.now() + data.expires_in * 1_000,
      tokenType: data.token_type,
      scope: data.scope,
    };
  }

  async revokeToken(token: string): Promise<void> {
    if (!this.config.revokeEndpoint) {
      throw new Error("Revoke endpoint not configured");
    }

    const body = new URLSearchParams({
      token,
      client_id: this.config.clientId,
    });

    if (this.config.clientSecret) {
      body.set("client_secret", this.config.clientSecret);
    }

    const res = await fetch(this.config.revokeEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body.toString(),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Token revocation failed (${res.status}): ${text}`);
    }
  }
}
