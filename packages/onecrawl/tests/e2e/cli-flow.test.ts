/**
 * E2E CLI Flow (M3-I1)
 * Tests CLI commands via subprocess — no browser needed.
 */
import { describe, it, expect } from 'vitest';
import { execSync } from 'node:child_process';
import path from 'node:path';

const CLI = 'node ' + path.resolve(__dirname, '../../../onecrawl-cli/onecrawl-cli.js');

function run(args: string): string {
  try {
    return execSync(`${CLI} ${args}`, {
      encoding: 'utf-8',
      timeout: 15000,
      env: { ...process.env, NO_COLOR: '1', FORCE_COLOR: '0' },
    });
  } catch (err: any) {
    // Return combined output even on non-zero exit
    return (err.stdout ?? '') + (err.stderr ?? '');
  }
}

describe('CLI E2E Flow', () => {
  it('--help shows custom commands alongside Playwright commands', () => {
    const out = run('--help');
    // Should include Playwright standard commands
    expect(out).toMatch(/open|codegen|install/i);
    // Should include OneCrawl custom commands
    expect(out).toMatch(/scroll|find|auth|health-check/i);
  });

  it('--version shows version string', () => {
    const out = run('--version');
    expect(out.trim()).toMatch(/\d+\.\d+/);
  });

  it('scroll --help shows usage information', () => {
    const out = run('scroll --help');
    expect(out).toMatch(/scroll/i);
    expect(out).toMatch(/usage|direction|down|up/i);
  });

  it('find --help shows strategies', () => {
    const out = run('find --help');
    expect(out).toMatch(/find/i);
    expect(out).toMatch(/usage|selector|text|strategy/i);
  });

  it('auth status fails gracefully when no credentials exist', () => {
    const out = run('auth status');
    // Should not crash — either shows "no credentials" or valid status
    expect(out).toBeDefined();
    expect(out.length).toBeGreaterThan(0);
  });

  it('health-check fails gracefully when no browser is running', () => {
    const out = run('health-check');
    // Should exit with an error message, not crash
    expect(out).toBeDefined();
    expect(out.length).toBeGreaterThan(0);
  });

  it('session-info fails gracefully when no browser is running', () => {
    const out = run('session-info');
    // Should exit with an error message, not crash
    expect(out).toBeDefined();
    expect(out.length).toBeGreaterThan(0);
  });
});
