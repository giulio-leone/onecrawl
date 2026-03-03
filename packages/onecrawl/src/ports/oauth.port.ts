/**
 * OAuth Port — platform-agnostic OAuth 2.1 interface.
 */

export interface OAuthTokens {
  accessToken: string;
  refreshToken?: string;
  expiresAt: number; // Unix timestamp ms
  tokenType: string;
  scope?: string;
}

export interface OAuthConfig {
  clientId: string;
  clientSecret?: string; // Optional for public clients (PKCE)
  redirectUri: string;
  authorizeEndpoint: string;
  tokenEndpoint: string;
  revokeEndpoint?: string;
  scopes: string[];
}

export interface OAuthPort {
  /** Build the authorization URL with PKCE challenge. */
  getAuthorizationUrl(state: string, codeChallenge: string): string;
  /** Exchange authorization code for tokens. */
  exchangeCode(code: string, codeVerifier: string): Promise<OAuthTokens>;
  /** Refresh an access token using a refresh token. */
  refreshToken(refreshToken: string): Promise<OAuthTokens>;
  /** Revoke a token. */
  revokeToken(token: string): Promise<void>;
}
