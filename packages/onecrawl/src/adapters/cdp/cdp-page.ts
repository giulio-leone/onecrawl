/**
 * CDP Page - Represents a browser tab with Chrome DevTools Protocol methods
 */

import { WebSocket } from "ws";
import type { CDPClient, CDPPageInfo } from "./client.js";

/** CDP message with ID (response). */
interface CDPMessage {
  id: number;
  result?: { result: { value: unknown } };
  error?: { code: number; message: string };
}

/**
 * CDPPage - Represents a browser tab with CDP methods
 */
export class CDPPage {
  private ws: WebSocket | null = null;
  private messageId = 0;
  private pendingCommands = new Map<
    number,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
    }
  >();

  constructor(
    private pageInfo: CDPPageInfo,
    private client: CDPClient,
  ) {}

  /** Connect to page. */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.pageInfo.webSocketDebuggerUrl);
      this.ws.on("open", async () => {
        await this.send("Page.enable");
        await this.send("Network.enable");
        await this.send("Runtime.enable");
        resolve();
      });
      this.ws.on("error", reject);
      this.ws.on("message", (data) => this.handleMessage(data.toString()));
    });
  }

  private handleMessage(data: string): void {
    const message = JSON.parse(data) as CDPMessage;
    if ("id" in message) {
      const pending = this.pendingCommands.get(message.id);
      if (pending) {
        this.pendingCommands.delete(message.id);
        if (message.error) {
          pending.reject(new Error(message.error.message));
        } else {
          pending.resolve(message.result);
        }
      }
    }
  }

  async send<T = unknown>(method: string, params?: object): Promise<T> {
    if (!this.ws) throw new Error("Page not connected");

    const id = ++this.messageId;
    return new Promise((resolve, reject) => {
      this.pendingCommands.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      this.ws!.send(JSON.stringify({ id, method, params }));
    });
  }

  /** Navigate to URL. */
  async goto(url: string, options?: { timeout?: number }): Promise<void> {
    const timeout = options?.timeout ?? 30000;
    await this.send("Page.navigate", { url });

    await Promise.race([
      new Promise<void>((resolve) => {
        const check = async () => {
          const state = await this.send<{ result: { value: string } }>(
            "Runtime.evaluate",
            { expression: "document.readyState" },
          );
          if (state.result.value === "complete") {
            resolve();
          } else {
            setTimeout(check, 100);
          }
        };
        check();
      }),
      new Promise<void>((_, reject) =>
        setTimeout(() => reject(new Error("Navigation timeout")), timeout),
      ),
    ]);
  }

  /** Get page HTML. */
  async getHTML(): Promise<string> {
    const result = await this.send<{ result: { value: string } }>(
      "Runtime.evaluate",
      { expression: "document.documentElement.outerHTML" },
    );
    return result.result.value;
  }

  /** Get page title. */
  async getTitle(): Promise<string> {
    const result = await this.send<{ result: { value: string } }>(
      "Runtime.evaluate",
      { expression: "document.title" },
    );
    return result.result.value;
  }

  /** Evaluate JavaScript. */
  async evaluate<T>(expression: string): Promise<T> {
    const result = await this.send<{ result: { value: T } }>(
      "Runtime.evaluate",
      { expression, returnByValue: true },
    );
    return result.result.value;
  }

  /** Set viewport. */
  async setViewport(width: number, height: number): Promise<void> {
    await this.send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: 1,
      mobile: false,
    });
  }

  /** Set user agent. */
  async setUserAgent(userAgent: string): Promise<void> {
    await this.send("Network.setUserAgentOverride", { userAgent });
  }

  /** Set cookies. */
  async setCookies(
    cookies: Array<{
      name: string;
      value: string;
      domain: string;
      path?: string;
    }>,
  ): Promise<void> {
    await this.send("Network.setCookies", { cookies });
  }

  /** Close page. */
  async close(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    await this.client.closePage(this.pageInfo.id);
  }
}
