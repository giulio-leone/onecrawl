/**
 * Auto-generated MCP tool definitions for all 41 CLI commands (M4–M9).
 * Each tool delegates to the onecrawl-server via `cli-exec`.
 */

import { z } from "zod";
import type { OneCrawlClient } from "@giulio-leone/onecrawl-client";

export interface McpToolDef {
  name: string;
  description: string;
  inputSchema: z.ZodObject<any>;
  handler: (
    args: Record<string, unknown>,
    client: OneCrawlClient,
  ) => Promise<{ content: Array<{ type: "text"; text: string }> }>;
}

// ---------------------------------------------------------------------------
// Helper: build a handler that maps tool name → CLI command via cli-exec
// ---------------------------------------------------------------------------
function makeHandler(
  toolName: string,
): McpToolDef["handler"] {
  const command = toolName.replace(/^cli_/, "").replace(/_/g, "-");
  return async (args, client) => {
    const result = await client.webAction("cli-exec", { command, ...args });
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result) }],
    };
  };
}

// ---------------------------------------------------------------------------
// M4 — Emulation (9 tools)
// ---------------------------------------------------------------------------

const cliViewport: McpToolDef = {
  name: "cli_viewport",
  description: "Set browser viewport dimensions, scale, or use a preset.",
  inputSchema: z.object({
    width: z.number().optional().describe("Viewport width in pixels"),
    height: z.number().optional().describe("Viewport height in pixels"),
    scale: z.number().optional().describe("Device scale factor"),
    preset: z
      .enum(["mobile", "tablet", "desktop"])
      .optional()
      .describe("Quick preset"),
  }),
  handler: makeHandler("cli_viewport"),
};

const cliDevice: McpToolDef = {
  name: "cli_device",
  description: "Emulate a known device by name, or list available devices.",
  inputSchema: z.object({
    name: z.string().optional().describe("Device name to emulate"),
    list: z.boolean().optional().describe("List all available device names"),
  }),
  handler: makeHandler("cli_device"),
};

const cliEmulateMedia: McpToolDef = {
  name: "cli_emulate_media",
  description:
    "Override media features: color scheme, reduced motion, media type.",
  inputSchema: z.object({
    colorScheme: z
      .enum(["dark", "light", "no-preference"])
      .optional()
      .describe("Preferred color scheme"),
    reducedMotion: z
      .enum(["reduce", "no-preference"])
      .optional()
      .describe("Reduced motion preference"),
    media: z
      .enum(["screen", "print"])
      .optional()
      .describe("CSS media type override"),
  }),
  handler: makeHandler("cli_emulate_media"),
};

const cliTimezone: McpToolDef = {
  name: "cli_timezone",
  description: "Set or reset the browser timezone.",
  inputSchema: z.object({
    timezone: z.string().optional().describe("IANA timezone id (e.g. Europe/Rome)"),
    reset: z.boolean().optional().describe("Reset to system default"),
  }),
  handler: makeHandler("cli_timezone"),
};

const cliLocale: McpToolDef = {
  name: "cli_locale",
  description: "Set or reset the browser locale.",
  inputSchema: z.object({
    locale: z.string().optional().describe("BCP-47 locale tag (e.g. it-IT)"),
    reset: z.boolean().optional().describe("Reset to system default"),
  }),
  handler: makeHandler("cli_locale"),
};

const cliUserAgent: McpToolDef = {
  name: "cli_user_agent",
  description: "Override or reset the browser user-agent string.",
  inputSchema: z.object({
    userAgent: z.string().optional().describe("Custom user-agent string"),
    reset: z.boolean().optional().describe("Reset to default"),
  }),
  handler: makeHandler("cli_user_agent"),
};

const cliOffline: McpToolDef = {
  name: "cli_offline",
  description: "Toggle or query offline network emulation.",
  inputSchema: z.object({
    enable: z.boolean().optional().describe("Go offline"),
    disable: z.boolean().optional().describe("Go online"),
    status: z.boolean().optional().describe("Query current offline state"),
  }),
  handler: makeHandler("cli_offline"),
};

const cliGeolocation: McpToolDef = {
  name: "cli_geolocation",
  description: "Set or reset geolocation coordinates.",
  inputSchema: z.object({
    latitude: z.number().optional().describe("Latitude"),
    longitude: z.number().optional().describe("Longitude"),
    accuracy: z.number().optional().describe("Accuracy in meters"),
    reset: z.boolean().optional().describe("Reset geolocation override"),
  }),
  handler: makeHandler("cli_geolocation"),
};

const cliPermissions: McpToolDef = {
  name: "cli_permissions",
  description: "Grant, deny, or list browser permissions.",
  inputSchema: z.object({
    action: z
      .enum(["grant", "deny", "list"])
      .optional()
      .describe("Permission action"),
    permission: z.string().optional().describe("Permission name (e.g. geolocation)"),
  }),
  handler: makeHandler("cli_permissions"),
};

// ---------------------------------------------------------------------------
// M5 — Network (10 tools)
// ---------------------------------------------------------------------------

const cliRoute: McpToolDef = {
  name: "cli_route",
  description:
    "Intercept network requests matching a pattern: abort or mock a response.",
  inputSchema: z.object({
    pattern: z.string().describe("URL glob or regex pattern to intercept"),
    action: z.enum(["abort", "mock"]).describe("Intercept action"),
    status: z.number().optional().describe("HTTP status for mock response"),
    body: z.string().optional().describe("Response body for mock"),
    contentType: z
      .string()
      .optional()
      .describe("Content-Type header for mock"),
  }),
  handler: makeHandler("cli_route"),
};

const cliUnroute: McpToolDef = {
  name: "cli_unroute",
  description: "Remove a previously registered network route.",
  inputSchema: z.object({
    pattern: z.string().optional().describe("Route pattern to remove"),
    all: z.boolean().optional().describe("Remove all routes"),
  }),
  handler: makeHandler("cli_unroute"),
};

const cliRequests: McpToolDef = {
  name: "cli_requests",
  description: "List captured network requests with optional filtering.",
  inputSchema: z.object({
    filter: z.string().optional().describe("URL substring filter"),
    type: z
      .enum(["xhr", "fetch", "all"])
      .optional()
      .describe("Request type filter"),
    clear: z.boolean().optional().describe("Clear captured requests"),
  }),
  handler: makeHandler("cli_requests"),
};

const cliHeaders: McpToolDef = {
  name: "cli_headers",
  description: "Set, list, or clear extra HTTP headers for all requests.",
  inputSchema: z.object({
    name: z.string().optional().describe("Header name"),
    value: z.string().optional().describe("Header value"),
    list: z.boolean().optional().describe("List current extra headers"),
    clear: z.boolean().optional().describe("Clear all extra headers"),
  }),
  handler: makeHandler("cli_headers"),
};

const cliHttpCredentials: McpToolDef = {
  name: "cli_http_credentials",
  description: "Set or clear HTTP Basic Auth credentials.",
  inputSchema: z.object({
    username: z.string().optional().describe("Username"),
    password: z.string().optional().describe("Password"),
    clear: z.boolean().optional().describe("Clear credentials"),
  }),
  handler: makeHandler("cli_http_credentials"),
};

const cliHar: McpToolDef = {
  name: "cli_har",
  description: "Start or stop HAR recording of network traffic.",
  inputSchema: z.object({
    action: z.enum(["start", "stop"]).describe("Start or stop recording"),
    file: z.string().optional().describe("Output HAR file path"),
  }),
  handler: makeHandler("cli_har"),
};

const cliTrace: McpToolDef = {
  name: "cli_trace",
  description: "Start or stop Playwright trace recording.",
  inputSchema: z.object({
    action: z.enum(["start", "stop"]).describe("Start or stop tracing"),
    file: z.string().optional().describe("Output trace file path"),
  }),
  handler: makeHandler("cli_trace"),
};

const cliProfiler: McpToolDef = {
  name: "cli_profiler",
  description: "Start or stop a JavaScript CPU profiler session.",
  inputSchema: z.object({
    action: z.enum(["start", "stop"]).describe("Start or stop profiling"),
    file: z.string().optional().describe("Output profile file path"),
  }),
  handler: makeHandler("cli_profiler"),
};

const cliConsole: McpToolDef = {
  name: "cli_console",
  description: "Read or clear captured browser console messages.",
  inputSchema: z.object({
    level: z
      .enum(["all", "log", "warn", "error", "info"])
      .optional()
      .describe("Filter by log level"),
    clear: z.boolean().optional().describe("Clear captured messages"),
  }),
  handler: makeHandler("cli_console"),
};

const cliJsErrors: McpToolDef = {
  name: "cli_js_errors",
  description: "List or clear captured uncaught JavaScript errors.",
  inputSchema: z.object({
    clear: z.boolean().optional().describe("Clear captured errors"),
  }),
  handler: makeHandler("cli_js_errors"),
};

// ---------------------------------------------------------------------------
// M6 — Frame / Tab (4 tools)
// ---------------------------------------------------------------------------

const cliFrame: McpToolDef = {
  name: "cli_frame",
  description: "Switch context to an iframe matching a selector.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the iframe element"),
  }),
  handler: makeHandler("cli_frame"),
};

const cliMainframe: McpToolDef = {
  name: "cli_mainframe",
  description: "Switch context back to the main (top-level) frame.",
  inputSchema: z.object({}),
  handler: makeHandler("cli_mainframe"),
};

const cliTab: McpToolDef = {
  name: "cli_tab",
  description: "List, open, switch, or close browser tabs.",
  inputSchema: z.object({
    action: z
      .enum(["list", "new", "switch", "close"])
      .describe("Tab operation"),
    index: z.number().optional().describe("Tab index for switch / close"),
    url: z.string().optional().describe("URL for new tab"),
  }),
  handler: makeHandler("cli_tab"),
};

const cliDialog: McpToolDef = {
  name: "cli_dialog",
  description: "Accept, dismiss, or query pending JavaScript dialogs.",
  inputSchema: z.object({
    action: z
      .enum(["accept", "dismiss", "status"])
      .optional()
      .describe("Dialog action"),
    text: z
      .string()
      .optional()
      .describe("Text to enter in a prompt dialog"),
  }),
  handler: makeHandler("cli_dialog"),
};

// ---------------------------------------------------------------------------
// M7 — Diff (3 tools)
// ---------------------------------------------------------------------------

const cliDiffSnapshot: McpToolDef = {
  name: "cli_diff_snapshot",
  description:
    "Take a DOM snapshot and diff against an optional baseline snapshot.",
  inputSchema: z.object({
    baseline: z.string().optional().describe("Path to baseline snapshot file"),
  }),
  handler: makeHandler("cli_diff_snapshot"),
};

const cliDiffScreenshot: McpToolDef = {
  name: "cli_diff_screenshot",
  description:
    "Take a screenshot and diff pixel-by-pixel against a baseline image.",
  inputSchema: z.object({
    baseline: z
      .string()
      .optional()
      .describe("Path to baseline screenshot file"),
    threshold: z
      .number()
      .optional()
      .describe("Pixel diff threshold (0-1, default 0.1)"),
    output: z
      .string()
      .optional()
      .describe("Path to save the diff image"),
  }),
  handler: makeHandler("cli_diff_screenshot"),
};

const cliDiffUrl: McpToolDef = {
  name: "cli_diff_url",
  description:
    "Compare two URLs by loading each and diffing their DOM or screenshots.",
  inputSchema: z.object({
    url1: z.string().describe("First URL"),
    url2: z.string().describe("Second URL"),
  }),
  handler: makeHandler("cli_diff_url"),
};

// ---------------------------------------------------------------------------
// M8 — Content (8 tools)
// ---------------------------------------------------------------------------

const cliSetContent: McpToolDef = {
  name: "cli_set_content",
  description: "Replace the page content with raw HTML.",
  inputSchema: z.object({
    html: z.string().describe("HTML content to set"),
    url: z.string().optional().describe("Virtual URL for the page"),
  }),
  handler: makeHandler("cli_set_content"),
};

const cliAddScript: McpToolDef = {
  name: "cli_add_script",
  description: "Inject a <script> tag into the current page.",
  inputSchema: z.object({
    code: z.string().describe("JavaScript code to inject"),
    type: z
      .enum(["module", "text/javascript"])
      .optional()
      .describe("Script type attribute"),
  }),
  handler: makeHandler("cli_add_script"),
};

const cliAddStyle: McpToolDef = {
  name: "cli_add_style",
  description: "Inject a <style> tag into the current page.",
  inputSchema: z.object({
    css: z.string().describe("CSS rules to inject"),
  }),
  handler: makeHandler("cli_add_style"),
};

const cliAddInitScript: McpToolDef = {
  name: "cli_add_init_script",
  description:
    "Register a script that runs before every page load (like a preload).",
  inputSchema: z.object({
    code: z.string().describe("JavaScript code to run before page scripts"),
  }),
  handler: makeHandler("cli_add_init_script"),
};

const cliPdf: McpToolDef = {
  name: "cli_pdf",
  description: "Export the current page to a PDF file.",
  inputSchema: z.object({
    file: z.string().optional().describe("Output file path"),
    format: z
      .enum(["Letter", "A4", "Legal"])
      .optional()
      .describe("Paper format"),
    landscape: z.boolean().optional().describe("Use landscape orientation"),
  }),
  handler: makeHandler("cli_pdf"),
};

const cliRecording: McpToolDef = {
  name: "cli_recording",
  description: "Start, stop, or restart a video recording of the browser.",
  inputSchema: z.object({
    action: z
      .enum(["start", "stop", "restart"])
      .describe("Recording action"),
    file: z.string().optional().describe("Output video file path"),
  }),
  handler: makeHandler("cli_recording"),
};

const cliScreencast: McpToolDef = {
  name: "cli_screencast",
  description: "Start or stop a screencast (frame-by-frame image sequence).",
  inputSchema: z.object({
    action: z.enum(["start", "stop"]).describe("Screencast action"),
    dir: z
      .string()
      .optional()
      .describe("Output directory for frame images"),
    quality: z
      .number()
      .optional()
      .describe("JPEG quality (0-100)"),
  }),
  handler: makeHandler("cli_screencast"),
};

const cliStorage: McpToolDef = {
  name: "cli_storage",
  description:
    "Get, set, list, clear, or remove keys in localStorage or sessionStorage.",
  inputSchema: z.object({
    action: z
      .enum(["get", "set", "list", "clear", "remove"])
      .describe("Storage operation"),
    key: z.string().optional().describe("Storage key"),
    value: z.string().optional().describe("Value to set"),
    type: z
      .enum(["local", "session"])
      .optional()
      .describe("Storage type (default: local)"),
  }),
  handler: makeHandler("cli_storage"),
};

const cliStorageState: McpToolDef = {
  name: "cli_storage_state",
  description:
    "Save or load full browser storage state (cookies + storage) to/from a file.",
  inputSchema: z.object({
    action: z.enum(["save", "load"]).describe("Save or load"),
    file: z.string().describe("File path for the storage state JSON"),
  }),
  handler: makeHandler("cli_storage_state"),
};

// ---------------------------------------------------------------------------
// M9 — Input (6 tools)
// ---------------------------------------------------------------------------

const cliKeyboard: McpToolDef = {
  name: "cli_keyboard",
  description:
    "Simulate keyboard actions: type, insertText, press, key down/up, or combos.",
  inputSchema: z.object({
    action: z
      .enum(["type", "inserttext", "press", "down", "up", "combo"])
      .describe("Keyboard action type"),
    text: z.string().describe("Text or key name (e.g. Enter, Ctrl+A)"),
  }),
  handler: makeHandler("cli_keyboard"),
};

const cliTap: McpToolDef = {
  name: "cli_tap",
  description: "Perform a touch-tap on an element or at coordinates.",
  inputSchema: z.object({
    selector: z.string().optional().describe("CSS selector to tap"),
    x: z.number().optional().describe("X coordinate"),
    y: z.number().optional().describe("Y coordinate"),
  }),
  handler: makeHandler("cli_tap"),
};

const cliClipboard: McpToolDef = {
  name: "cli_clipboard",
  description: "Read, write, or clear the browser clipboard.",
  inputSchema: z.object({
    action: z
      .enum(["read", "write", "clear"])
      .describe("Clipboard operation"),
    text: z
      .string()
      .optional()
      .describe("Text to write (when action is write)"),
  }),
  handler: makeHandler("cli_clipboard"),
};

const cliGetStyles: McpToolDef = {
  name: "cli_get_styles",
  description: "Get computed CSS styles for an element.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the target element"),
    properties: z
      .string()
      .optional()
      .describe("Comma-separated list of CSS properties to return"),
  }),
  handler: makeHandler("cli_get_styles"),
};

const cliGetBox: McpToolDef = {
  name: "cli_get_box",
  description:
    "Get the bounding box (x, y, width, height) of an element.",
  inputSchema: z.object({
    selector: z.string().describe("CSS selector of the target element"),
  }),
  handler: makeHandler("cli_get_box"),
};

const cliWaitForFunction: McpToolDef = {
  name: "cli_wait_for_function",
  description:
    "Wait until a JavaScript expression evaluates to a truthy value.",
  inputSchema: z.object({
    expression: z.string().describe("JS expression to evaluate repeatedly"),
    timeout: z
      .number()
      .optional()
      .describe("Max wait time in milliseconds"),
  }),
  handler: makeHandler("cli_wait_for_function"),
};

// ---------------------------------------------------------------------------
// M13 — Agent-Browser Parity (7 tools)
// ---------------------------------------------------------------------------

const cliSnapshot: McpToolDef = {
  name: "cli_snapshot",
  description:
    "Get the page accessibility tree with element refs — the key AI-agent command for compact, token-efficient page understanding.",
  inputSchema: z.object({
    interactive: z
      .boolean()
      .optional()
      .describe("Show only interactive elements (buttons, links, inputs)"),
    compact: z.boolean().optional().describe("Compact single-line output"),
    depth: z
      .number()
      .optional()
      .describe("Max tree depth to traverse"),
    selector: z
      .string()
      .optional()
      .describe("CSS selector to scope the snapshot subtree"),
    json: z.boolean().optional().describe("Output as JSON instead of text"),
  }),
  handler: makeHandler("cli_snapshot"),
};

const cliDblclick: McpToolDef = {
  name: "cli_dblclick",
  description: "Double-click an element by ref number or CSS selector.",
  inputSchema: z.object({
    ref: z
      .string()
      .describe("Element ref number or CSS selector to double-click"),
    force: z.boolean().optional().describe("Force the double-click"),
  }),
  handler: makeHandler("cli_dblclick"),
};

const cliFocus: McpToolDef = {
  name: "cli_focus",
  description: "Focus an element by ref number or CSS selector.",
  inputSchema: z.object({
    ref: z
      .string()
      .describe("Element ref number or CSS selector to focus"),
  }),
  handler: makeHandler("cli_focus"),
};

const cliCheck: McpToolDef = {
  name: "cli_check",
  description:
    "Check or uncheck a checkbox/radio element by ref number or CSS selector.",
  inputSchema: z.object({
    ref: z
      .string()
      .describe("Element ref number or CSS selector"),
    uncheck: z
      .boolean()
      .optional()
      .describe("Uncheck instead of check"),
  }),
  handler: makeHandler("cli_check"),
};

const cliScrollintoview: McpToolDef = {
  name: "cli_scrollintoview",
  description:
    "Scroll an element into the visible viewport by ref number or CSS selector.",
  inputSchema: z.object({
    ref: z
      .string()
      .describe("Element ref number or CSS selector to scroll into view"),
  }),
  handler: makeHandler("cli_scrollintoview"),
};

const cliConnect: McpToolDef = {
  name: "cli_connect",
  description:
    "Connect to a browser via Chrome DevTools Protocol port for remote automation.",
  inputSchema: z.object({
    port: z.number().describe("CDP port number (1-65535)"),
  }),
  handler: makeHandler("cli_connect"),
};

const cliHighlight: McpToolDef = {
  name: "cli_highlight",
  description:
    "Visually highlight an element with a colored border and overlay for debugging.",
  inputSchema: z.object({
    ref: z
      .string()
      .describe("Element ref number or CSS selector to highlight"),
    color: z
      .string()
      .optional()
      .describe("Highlight color (default: red)"),
    duration: z
      .number()
      .optional()
      .describe("Duration in ms before removing highlight (default: 2000)"),
  }),
  handler: makeHandler("cli_highlight"),
};

// ---------------------------------------------------------------------------
// Export all 48 generated tools
// ---------------------------------------------------------------------------

export const generatedCliTools: McpToolDef[] = [
  // M4 — Emulation (9)
  cliViewport,
  cliDevice,
  cliEmulateMedia,
  cliTimezone,
  cliLocale,
  cliUserAgent,
  cliOffline,
  cliGeolocation,
  cliPermissions,
  // M5 — Network (10)
  cliRoute,
  cliUnroute,
  cliRequests,
  cliHeaders,
  cliHttpCredentials,
  cliHar,
  cliTrace,
  cliProfiler,
  cliConsole,
  cliJsErrors,
  // M6 — Frame/Tab (4)
  cliFrame,
  cliMainframe,
  cliTab,
  cliDialog,
  // M7 — Diff (3)
  cliDiffSnapshot,
  cliDiffScreenshot,
  cliDiffUrl,
  // M8 — Content (8+1 storage-state = 9)
  cliSetContent,
  cliAddScript,
  cliAddStyle,
  cliAddInitScript,
  cliPdf,
  cliRecording,
  cliScreencast,
  cliStorage,
  cliStorageState,
  // M9 — Input (6)
  cliKeyboard,
  cliTap,
  cliClipboard,
  cliGetStyles,
  cliGetBox,
  cliWaitForFunction,
  // M13 — Agent-Browser Parity (7)
  cliSnapshot,
  cliDblclick,
  cliFocus,
  cliCheck,
  cliScrollintoview,
  cliConnect,
  cliHighlight,
];
