#!/usr/bin/env node

/**
 * onecrawl-ptc CLI — run or generate PTC scripts.
 *
 * Usage:
 *   onecrawl-ptc run <script.js> [--session=<s>] [--url=<u>] [--max-attempts=3]
 *   onecrawl-ptc generate "<task>" [--provider=claude|openai|gemini]
 */

import { readFile } from "node:fs/promises";
import { OneCrawlClient } from "@giulio-leone/onecrawl-client";
import { PtcEngine } from "./engine.js";
import { buildToolRegistry } from "./tool-registry.js";

function usage(): never {
  // eslint-disable-next-line no-console
  console.log(`Usage:
  onecrawl-ptc run <script.js>  [--session=default] [--url=http://localhost:8931] [--max-attempts=3]
  onecrawl-ptc generate "<task>" [--provider=claude|openai|gemini]`);
  process.exit(1);
}

function parseFlag(args: string[], flag: string, def: string): string {
  const prefix = `--${flag}=`;
  const entry = args.find((a) => a.startsWith(prefix));
  return entry ? entry.slice(prefix.length) : def;
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 2) usage();

  const command = args[0];
  const subject = args[1];

  const client = new OneCrawlClient({
    baseUrl: parseFlag(args, "url", "http://localhost:8931"),
  });
  const registry = buildToolRegistry();

  // Placeholder LLM generate — users should override or pipe from stdin
  const llmGenerate = async (prompt: string): Promise<string> => {
    // eslint-disable-next-line no-console
    console.log("[PTC] LLM prompt generated. Paste your response:");
    // eslint-disable-next-line no-console
    console.log(prompt);
    throw new Error("Interactive LLM generation not yet supported in CLI — provide a script file instead.");
  };

  const engine = new PtcEngine({ client, llmGenerate, registry });

  if (command === "run") {
    const scriptContent = await readFile(subject, "utf-8");
    const result = await engine.run(scriptContent, {
      session: parseFlag(args, "session", "default"),
      maxAttempts: Number(parseFlag(args, "max-attempts", "3")),
    });
    // eslint-disable-next-line no-console
    console.log(JSON.stringify(result, null, 2));
    process.exit(result.success ? 0 : 1);
  } else if (command === "generate") {
    const script = await engine.generate({
      task: subject,
      provider: parseFlag(args, "provider", "claude") as "claude" | "openai" | "gemini",
    });
    // eslint-disable-next-line no-console
    console.log(script);
  } else {
    usage();
  }
}

main().catch((err) => {
  // eslint-disable-next-line no-console
  console.error("Fatal:", err);
  process.exit(1);
});
