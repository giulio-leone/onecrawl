/**
 * Proxy Manager - Rotating proxy support for anti-bot bypass
 */

/** Proxy configuration */
export interface ProxyConfig {
  host: string;
  port: number;
  username?: string;
  password?: string;
  protocol?: "http" | "https" | "socks5";
}

/** Proxy provider interface */
export interface ProxyProvider {
  getProxy(): Promise<ProxyConfig>;
  reportFailure(proxy: ProxyConfig): void;
  reportSuccess(proxy: ProxyConfig): void;
}

/** Proxy health status */
interface ProxyHealth {
  proxy: ProxyConfig;
  successCount: number;
  failureCount: number;
  lastUsed: number;
  avgLatency: number;
}

/**
 * Static proxy pool with health monitoring
 */
export class StaticProxyPool implements ProxyProvider {
  private proxies: ProxyHealth[] = [];
  private currentIndex = 0;

  constructor(proxies: ProxyConfig[]) {
    this.proxies = proxies.map((proxy) => ({
      proxy,
      successCount: 0,
      failureCount: 0,
      lastUsed: 0,
      avgLatency: 0,
    }));
  }

  async getProxy(): Promise<ProxyConfig> {
    if (this.proxies.length === 0) {
      throw new Error("No proxies available");
    }

    // Round-robin with health weighting
    const healthy = this.proxies.filter(
      (p) => p.failureCount < 5 || p.successCount > p.failureCount,
    );

    if (healthy.length === 0) {
      // Reset all proxies if all failed
      for (const p of this.proxies) {
        p.failureCount = 0;
      }
      return this.proxies[0]!.proxy;
    }

    // Sort by success rate and latency
    healthy.sort((a, b) => {
      const aRate = a.successCount / (a.successCount + a.failureCount + 1);
      const bRate = b.successCount / (b.successCount + b.failureCount + 1);
      if (Math.abs(aRate - bRate) > 0.1) return bRate - aRate;
      return a.avgLatency - b.avgLatency;
    });

    const selected = healthy[this.currentIndex % healthy.length]!;
    this.currentIndex++;
    selected.lastUsed = Date.now();

    return selected.proxy;
  }

  reportFailure(proxy: ProxyConfig): void {
    const health = this.findHealth(proxy);
    if (health) health.failureCount++;
  }

  reportSuccess(proxy: ProxyConfig, latency?: number): void {
    const health = this.findHealth(proxy);
    if (health) {
      health.successCount++;
      if (latency) {
        health.avgLatency =
          (health.avgLatency * (health.successCount - 1) + latency) /
          health.successCount;
      }
    }
  }

  private findHealth(proxy: ProxyConfig): ProxyHealth | undefined {
    return this.proxies.find(
      (p) => p.proxy.host === proxy.host && p.proxy.port === proxy.port,
    );
  }

  getStats(): Array<{
    host: string;
    port: number;
    successRate: number;
    avgLatency: number;
  }> {
    return this.proxies.map((p) => ({
      host: p.proxy.host,
      port: p.proxy.port,
      successRate: p.successCount / (p.successCount + p.failureCount + 1),
      avgLatency: p.avgLatency,
    }));
  }
}

/**
 * Rotating proxy service adapter (Bright Data, Oxylabs, etc.)
 */
export class RotatingProxyService implements ProxyProvider {
  private config: ProxyConfig;

  constructor(config: {
    host: string;
    port: number;
    username: string;
    password: string;
  }) {
    this.config = {
      ...config,
      protocol: "http",
    };
  }

  async getProxy(): Promise<ProxyConfig> {
    // Rotating proxy services rotate IPs on each request
    // The same proxy endpoint gives different IPs
    return this.config;
  }

  reportFailure(): void {
    // Rotating services handle this automatically
  }

  reportSuccess(): void {
    // Rotating services handle this automatically
  }
}

/**
 * Convert proxy config to URL string
 */
export function proxyToUrl(proxy: ProxyConfig): string {
  const protocol = proxy.protocol ?? "http";
  const auth =
    proxy.username && proxy.password
      ? `${encodeURIComponent(proxy.username)}:${encodeURIComponent(proxy.password)}@`
      : "";

  return `${protocol}://${auth}${proxy.host}:${proxy.port}`;
}

/**
 * Parse proxy URL to config
 */
export function parseProxyUrl(url: string): ProxyConfig {
  const parsed = new URL(url);

  return {
    host: parsed.hostname,
    port: parseInt(parsed.port, 10) || 80,
    username: parsed.username || undefined,
    password: parsed.password || undefined,
    protocol: parsed.protocol.replace(":", "") as ProxyConfig["protocol"],
  };
}
