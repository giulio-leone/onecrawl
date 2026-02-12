/**
 * CDP (Chrome DevTools Protocol) Client
 * Direct communication with Chrome/Chromium without Playwright overhead.
 */

import { WebSocket } from "ws";
import { spawn, type ChildProcess } from "child_process";
import { findChrome } from "./chrome-finder.js";

interface CDPResponse {
  id: number;
  result?: unknown;
  error?: { code: number; message: string };
}
interface CDPEvent {
  method: string;
  params: unknown;
}

/** Page info from CDP */
export interface CDPPageInfo {
  id: string;
  type: string;
  url: string;
  webSocketDebuggerUrl: string;
}

/** CDP connection options */
export interface CDPClientOptions {
  executablePath?: string;
  port?: number;
  headless?: boolean;
  userDataDir?: string;
  args?: string[];
}

const CHROME_FLAGS = [
  "--no-first-run",
  "--no-default-browser-check",
  "--disable-background-networking",
  "--disable-client-side-phishing-detection",
  "--disable-default-apps",
  "--disable-extensions",
  "--disable-hang-monitor",
  "--disable-popup-blocking",
  "--disable-prompt-on-repost",
  "--disable-sync",
  "--disable-translate",
  "--metrics-recording-only",
  "--safebrowsing-disable-auto-update",
];

/** Lightweight CDP Client - Direct Chrome DevTools Protocol */
export class CDPClient {
  private ws: WebSocket | null = null;
  private browserProcess: ChildProcess | null = null;
  private messageId = 0;
  private pendingCommands = new Map<
    number,
    { resolve: (value: unknown) => void; reject: (error: Error) => void }
  >();
  private eventHandlers = new Map<string, ((params: unknown) => void)[]>();
  private wsUrl: string | null = null;
  private port: number;

  constructor(private options: CDPClientOptions = {}) {
    this.port = options.port ?? 9222;
  }

  async launch(): Promise<void> {
    const executablePath = this.options.executablePath ?? (await findChrome());
    const args = [
      `--remote-debugging-port=${this.port}`,
      ...CHROME_FLAGS,
      ...(this.options.headless !== false ? ["--headless=new"] : []),
      ...(this.options.userDataDir
        ? [`--user-data-dir=${this.options.userDataDir}`]
        : []),
      ...(this.options.args ?? []),
    ];

    this.browserProcess = spawn(executablePath, args, {
      stdio: ["ignore", "pipe", "pipe"],
    });
    await this.waitForBrowser();
  }

  async connect(wsUrl?: string): Promise<void> {
    if (wsUrl) {
      this.wsUrl = wsUrl;
    } else {
      const response = await fetch(
        `http://localhost:${this.port}/json/version`,
      );
      const data = (await response.json()) as { webSocketDebuggerUrl: string };
      this.wsUrl = data.webSocketDebuggerUrl;
    }
    await this.connectWebSocket();
  }

  private async waitForBrowser(): Promise<void> {
    for (let i = 0; i < 30; i++) {
      try {
        const response = await fetch(
          `http://localhost:${this.port}/json/version`,
        );
        if (response.ok) {
          const data = (await response.json()) as {
            webSocketDebuggerUrl: string;
          };
          this.wsUrl = data.webSocketDebuggerUrl;
          await this.connectWebSocket();
          return;
        }
      } catch {
        await new Promise((r) => setTimeout(r, 100));
      }
    }
    throw new Error("Failed to connect to Chrome");
  }

  private async connectWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.wsUrl!);
      this.ws.on("open", () => resolve());
      this.ws.on("error", reject);
      this.ws.on("message", (data) => this.handleMessage(data.toString()));
    });
  }

  private handleMessage(data: string): void {
    const message = JSON.parse(data) as CDPResponse | CDPEvent;
    if ("id" in message) {
      const pending = this.pendingCommands.get(message.id);
      if (pending) {
        this.pendingCommands.delete(message.id);
        if (message.error) pending.reject(new Error(message.error.message));
        else pending.resolve(message.result);
      }
    } else if ("method" in message) {
      this.eventHandlers.get(message.method)?.forEach((h) => h(message.params));
    }
  }

  async send<T = unknown>(method: string, params?: object): Promise<T> {
    if (!this.ws) throw new Error("Not connected");
    const id = ++this.messageId;
    return new Promise((resolve, reject) => {
      this.pendingCommands.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      this.ws!.send(JSON.stringify({ id, method, params }));
    });
  }

  on(method: string, handler: (params: unknown) => void): void {
    const handlers = this.eventHandlers.get(method) ?? [];
    handlers.push(handler);
    this.eventHandlers.set(method, handlers);
  }

  async newPage(): Promise<CDPPageInfo> {
    const response = await fetch(`http://localhost:${this.port}/json/new`);
    return response.json() as Promise<CDPPageInfo>;
  }

  async getPages(): Promise<CDPPageInfo[]> {
    const response = await fetch(`http://localhost:${this.port}/json/list`);
    return response.json() as Promise<CDPPageInfo[]>;
  }

  async closePage(pageId: string): Promise<void> {
    await fetch(`http://localhost:${this.port}/json/close/${pageId}`);
  }

  async close(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    if (this.browserProcess) {
      this.browserProcess.kill();
      this.browserProcess = null;
    }
  }

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}

export { CDPPage } from "./cdp-page.js";
