/**
 * Chrome executable finder - platform-specific Chrome path detection
 */

/** Find Chrome executable on the current platform. */
export async function findChrome(): Promise<string> {
  const paths = [
    // macOS
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    // Linux
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    // Windows
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  ];

  try {
    const { existsSync } = await import("fs");
    for (const p of paths) {
      if (existsSync(p)) return p;
    }
  } catch {
    // fs not available (React Native)
  }

  throw new Error(
    "Chrome not found. Install Chrome or provide executablePath.",
  );
}
