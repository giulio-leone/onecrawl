/**
 * @giulio-leone/onecrawl-client
 *
 * Zero-dependency HTTP client for OneCrawl server.
 * Works in Node.js, Deno, and modern browsers.
 */

// ── Types ────────────────────────────────────────────────────────────────────

export type SearchEngine = "google" | "bing" | "duckduckgo";

export interface SearchOptions {
  limit?: number;
  engines?: SearchEngine[];
  extractImages?: boolean;
}

export interface SearchResult {
  title: string;
  url: string;
  snippet?: string;
  imageUrl?: string;
  images?: CrawledImage[];
}

export interface SearchResponse {
  results: SearchResult[];
}

export interface CrawledImage {
  filename: string;
  originalUrl: string;
  alt?: string;
  width?: number;
  height?: number;
  sizeBytes: number;
  localPath: string;
}

export interface IngestOptions {
  extractImages?: boolean;
  sourceUrl?: string;
  workspaceId?: string;
}

export interface IngestResponse {
  title: string;
  content: string;
  url: string;
  images?: CrawledImage[];
  [key: string]: unknown;
}

export interface ScrapeOptions {
  profileId: string;
  waitForSelector?: string;
  timeout?: number;
  headless?: boolean;
  cdpUrl?: string;
}

export interface ScrapeResponse {
  url: string;
  title: string;
  body: string;
  screenshotBase64: string | null;
}

export interface HealthResponse {
  status: string;
  service: string;
  version: string;
  profiles: number;
  webSessions: number;
}

// ── Profile Types ────────────────────────────────────────────────────────────

export interface Profile {
  id: string;
  name: string;
  services: string[];
  headless: boolean;
  [key: string]: unknown;
}

export interface CreateProfileData {
  name: string;
  services?: string[];
  headless?: boolean;
}

export interface LoginOptions {
  service: string;
  headless?: boolean;
  timeoutMs?: number;
}

export interface LoginResult {
  success: boolean;
  [key: string]: unknown;
}

export interface VerifyResult {
  valid: boolean;
  service: string;
  profileId: string;
}

// ── Web Action Types ─────────────────────────────────────────────────────────

export type WebAction =
  | "navigate"
  | "click"
  | "type"
  | "press"
  | "cookies/get"
  | "cookies/set"
  | "screenshot"
  | "upload"
  | "wait"
  | "extract"
  | "evaluate"
  | "human/warmup"
  | "human/click"
  | "human/type";

export interface WebActionResult {
  [key: string]: unknown;
}

export interface WebSession {
  id: string;
  [key: string]: unknown;
}

// ── CLI Types ────────────────────────────────────────────────────────────────

export interface CliExecuteOptions {
  session?: string;
  timeout?: number;
}

export interface CliExecuteResult {
  exitCode: number;
  stdout?: string;
  stderr?: string;
  [key: string]: unknown;
}

export interface CliBatchCommand {
  command: string;
  args?: string[];
}

export interface CliBatchResult {
  session: string;
  results: Array<CliBatchCommand & CliExecuteResult>;
  allSucceeded: boolean;
}

// ── Error ────────────────────────────────────────────────────────────────────

export class OneCrawlError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly body?: unknown,
  ) {
    super(message);
    this.name = "OneCrawlError";
  }
}

// ── Client Options ───────────────────────────────────────────────────────────

export type FetchFn = typeof globalThis.fetch;

export interface OneCrawlClientOptions {
  /** Custom fetch implementation (defaults to globalThis.fetch) */
  fetchFn?: FetchFn;
  /** Default timeout in ms for all requests (defaults to 30_000) */
  timeout?: number;
  /** Default headers to include in every request */
  headers?: Record<string, string>;
}

// ── Client ───────────────────────────────────────────────────────────────────

export class OneCrawlClient {
  private readonly baseUrl: string;
  private readonly fetchFn: FetchFn;
  private readonly timeout: number;
  private readonly defaultHeaders: Record<string, string>;

  public readonly profiles: ProfileClient;
  public readonly cli: CliClient;

  constructor(baseUrl: string, options: OneCrawlClientOptions = {}) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.fetchFn = options.fetchFn ?? globalThis.fetch.bind(globalThis);
    this.timeout = options.timeout ?? 30_000;
    this.defaultHeaders = options.headers ?? {};
    this.profiles = new ProfileClient(this);
    this.cli = new CliClient(this);
  }

  // ── Core Endpoints ─────────────────────────────────────────────────────

  /** POST /search — search the web via configured engines */
  async search(query: string, options: SearchOptions = {}): Promise<SearchResponse> {
    return this.post<SearchResponse>("/search", {
      query,
      limit: options.limit ?? 10,
      engines: options.engines,
      extractImages: options.extractImages,
    });
  }

  /** POST /ingest — extract content from a URL or query */
  async ingest(query: string, options: IngestOptions = {}): Promise<IngestResponse> {
    return this.post<IngestResponse>("/ingest", {
      mode: "free",
      query,
      extractImages: options.extractImages,
      sourceUrl: options.sourceUrl,
      workspaceId: options.workspaceId ?? "default",
    });
  }

  /** POST /scrape — browser-based page scraping */
  async scrape(url: string, options: ScrapeOptions): Promise<ScrapeResponse> {
    return this.post<ScrapeResponse>("/scrape", { url, ...options });
  }

  /** GET /health — server health check */
  async health(): Promise<HealthResponse> {
    return this.get<HealthResponse>("/health");
  }

  // ── Web Actions ────────────────────────────────────────────────────────

  /** POST /web/{action} — execute a browser web action */
  async webAction(action: WebAction, params: Record<string, unknown> = {}): Promise<WebActionResult> {
    return this.post<WebActionResult>(`/web/${action}`, params);
  }

  /** GET /web/sessions — list active browser sessions */
  async listSessions(): Promise<{ sessions: WebSession[] }> {
    return this.get<{ sessions: WebSession[] }>("/web/sessions");
  }

  /** DELETE /web/sessions/:id — close a browser session */
  async closeSession(sessionId: string): Promise<{ closed: boolean }> {
    return this.del<{ closed: boolean }>(`/web/sessions/${encodeURIComponent(sessionId)}`);
  }

  /** POST /web/connect — connect to a CDP endpoint */
  async connectCDP(params: Record<string, unknown>): Promise<WebActionResult> {
    return this.post<WebActionResult>("/web/connect", params);
  }

  // ── HTTP Internals ─────────────────────────────────────────────────────

  /** @internal */
  async get<T>(path: string): Promise<T> {
    const res = await this.fetchFn(`${this.baseUrl}${path}`, {
      method: "GET",
      headers: { ...this.defaultHeaders },
      signal: AbortSignal.timeout(this.timeout),
    });
    return this.handleResponse<T>(res);
  }

  /** @internal */
  async post<T>(path: string, body: unknown): Promise<T> {
    const res = await this.fetchFn(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json", ...this.defaultHeaders },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(this.timeout),
    });
    return this.handleResponse<T>(res);
  }

  /** @internal */
  async del<T>(path: string): Promise<T> {
    const res = await this.fetchFn(`${this.baseUrl}${path}`, {
      method: "DELETE",
      headers: { ...this.defaultHeaders },
      signal: AbortSignal.timeout(this.timeout),
    });
    return this.handleResponse<T>(res);
  }

  private async handleResponse<T>(res: Response): Promise<T> {
    const text = await res.text();
    if (!res.ok) {
      let body: unknown;
      try {
        body = JSON.parse(text);
      } catch {
        body = text;
      }
      throw new OneCrawlError(
        `OneCrawl request failed: ${res.status} ${res.statusText}`,
        res.status,
        body,
      );
    }
    try {
      return JSON.parse(text) as T;
    } catch {
      throw new OneCrawlError("Invalid JSON response from OneCrawl", 502, text.slice(0, 500));
    }
  }
}

// ── Profile Sub-Client ───────────────────────────────────────────────────────

class ProfileClient {
  constructor(private readonly client: OneCrawlClient) {}

  /** GET /profiles — list all profiles */
  async list(): Promise<{ profiles: Profile[] }> {
    return this.client.get<{ profiles: Profile[] }>("/profiles");
  }

  /** GET /profiles/:id — get a single profile */
  async get(id: string): Promise<{ profile: Profile }> {
    return this.client.get<{ profile: Profile }>(`/profiles/${id}`);
  }

  /** POST /profiles — create a new profile */
  async create(data: CreateProfileData): Promise<{ profile: Profile }> {
    return this.client.post<{ profile: Profile }>("/profiles", data);
  }

  /** DELETE /profiles/:id — delete a profile */
  async delete(id: string): Promise<{ deleted: boolean }> {
    return this.client.del<{ deleted: boolean }>(`/profiles/${id}`);
  }

  /** POST /profiles/:id/login — login a service */
  async login(id: string, options: LoginOptions): Promise<{ result: LoginResult }> {
    return this.client.post<{ result: LoginResult }>(`/profiles/${id}/login`, options);
  }

  /** GET /profiles/:id/verify/:service — verify session */
  async verify(id: string, service: string): Promise<VerifyResult> {
    return this.client.get<VerifyResult>(`/profiles/${id}/verify/${service}`);
  }
}

// ── CLI Sub-Client ───────────────────────────────────────────────────────────

class CliClient {
  constructor(private readonly client: OneCrawlClient) {}

  /** POST /cli/execute — execute a CLI command */
  async execute(
    command: string,
    args?: string[],
    options?: CliExecuteOptions,
  ): Promise<CliExecuteResult> {
    return this.client.post<CliExecuteResult>("/cli/execute", {
      command,
      args,
      ...options,
    });
  }

  /** POST /cli/batch — execute multiple CLI commands */
  async batch(
    session: string,
    commands: CliBatchCommand[],
    timeout?: number,
  ): Promise<CliBatchResult> {
    return this.client.post<CliBatchResult>("/cli/batch", {
      session,
      commands,
      timeout,
    });
  }

  /** GET /cli/health — check CLI engine health */
  async health(): Promise<{ status: string; engine: string; exitCode: number }> {
    return this.client.get<{ status: string; engine: string; exitCode: number }>("/cli/health");
  }
}
