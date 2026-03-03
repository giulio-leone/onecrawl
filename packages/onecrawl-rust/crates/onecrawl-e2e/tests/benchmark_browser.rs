//! Real browser E2E benchmark: chromiumoxide vs playwright-rs
//!
//! Run: cargo test -p onecrawl-e2e --test benchmark_browser --features onecrawl-cdp/playwright -- --nocapture
//! Or without playwright: cargo test -p onecrawl-e2e --test benchmark_browser -- --nocapture

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn report_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("reports")
        .join("benchmark");
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ─── Chromiumoxide Benchmarks ───

#[tokio::test]
async fn bench_chromiumoxide_launch() {
    use onecrawl_cdp::browser::BrowserSession;

    let start = Instant::now();
    let session = BrowserSession::launch_headless().await;
    let elapsed = start.elapsed();

    match session {
        Ok(_session) => {
            println!("✅ chromiumoxide launch: {:.0}ms", elapsed.as_millis());
            // Save timing
            let dir = report_dir();
            fs::write(
                dir.join("chromiumoxide_launch_ms.txt"),
                format!("{}", elapsed.as_millis()),
            )
            .unwrap();
        }
        Err(e) => {
            println!("❌ chromiumoxide launch failed: {e}");
            println!("   (requires Chrome/Chromium installed)");
        }
    }
}

#[tokio::test]
async fn bench_chromiumoxide_navigate_screenshot() {
    use onecrawl_cdp::browser::BrowserSession;
    use onecrawl_cdp::navigation;
    use onecrawl_cdp::screenshot;

    let session = match BrowserSession::launch_headless().await {
        Ok(s) => s,
        Err(e) => {
            println!("⚠️  skipping (no browser): {e}");
            return;
        }
    };

    let page = match session.new_page().await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️  skipping (new_page failed): {e}");
            return;
        }
    };

    // Navigation benchmark
    let nav_start = Instant::now();
    let nav_result = navigation::goto(&page, "https://example.com").await;
    let nav_elapsed = nav_start.elapsed();

    match nav_result {
        Ok(_) => println!(
            "✅ chromiumoxide navigate (example.com): {:.0}ms",
            nav_elapsed.as_millis()
        ),
        Err(e) => {
            println!("❌ chromiumoxide navigate failed: {e}");
            return;
        }
    }

    // Screenshot benchmark
    let ss_start = Instant::now();
    let ss_result = screenshot::screenshot_full(&page).await;
    let ss_elapsed = ss_start.elapsed();

    match ss_result {
        Ok(bytes) => {
            println!(
                "✅ chromiumoxide screenshot: {:.0}ms ({} bytes)",
                ss_elapsed.as_millis(),
                bytes.len()
            );
            let dir = report_dir();
            fs::write(dir.join("chromiumoxide_example.png"), &bytes).unwrap();
            fs::write(
                dir.join("chromiumoxide_nav_ms.txt"),
                format!("{}", nav_elapsed.as_millis()),
            )
            .unwrap();
            fs::write(
                dir.join("chromiumoxide_screenshot_ms.txt"),
                format!("{}", ss_elapsed.as_millis()),
            )
            .unwrap();
        }
        Err(e) => println!("❌ chromiumoxide screenshot failed: {e}"),
    }

    // Get page title
    let title = page
        .evaluate("document.title")
        .await
        .map(|v: String| v)
        .unwrap_or_default();
    println!("   Page title: {title}");
}

#[tokio::test]
async fn bench_chromiumoxide_stealth() {
    use onecrawl_cdp::browser::BrowserSession;
    use onecrawl_cdp::navigation;
    use onecrawl_cdp::stealth::fingerprint::Fingerprint;
    use onecrawl_cdp::stealth::scripts::get_stealth_init_script;

    let session = match BrowserSession::launch_headless().await {
        Ok(s) => s,
        Err(e) => {
            println!("⚠️  skipping stealth test (no browser): {e}");
            return;
        }
    };

    let page = match session.new_page().await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️  skipping (new_page failed): {e}");
            return;
        }
    };

    // Generate stealth fingerprint and script
    let fp = Fingerprint::random();
    let script = get_stealth_init_script(&fp);

    println!("🔒 Stealth fingerprint generated:");
    println!("   Platform: {}", fp.platform);
    println!("   Languages: {:?}", fp.languages);
    println!("   HW Concurrency: {}", fp.hardware_concurrency);
    println!("   Device Memory: {}", fp.device_memory);
    println!("   WebGL Vendor: {}", fp.webgl_vendor);
    println!("   Script size: {} chars", script.len());

    // Inject stealth script before navigation
    let inject_start = Instant::now();
    let inject_result = page
        .evaluate(format!(
            "(function() {{ {script}; return 'stealth_injected'; }})()"
        ))
        .await;
    let inject_elapsed = inject_start.elapsed();

    match inject_result {
        Ok(result) => {
            let result_str: String = result;
            println!(
                "✅ Stealth injection: {:.0}ms (result: {result_str})",
                inject_elapsed.as_millis()
            );
        }
        Err(e) => {
            println!("❌ Stealth injection failed: {e}");
            return;
        }
    }

    // Navigate to a test page and verify patches
    let _ = navigation::goto(&page, "https://example.com").await;

    // Re-inject after navigation (simulate addScriptToEvaluateOnNewDocument)
    let _ = page
        .evaluate(format!("(function() {{ {script} }})()",))
        .await;

    // Verify stealth patches
    let checks = vec![
        ("navigator.webdriver", "false"),
        ("typeof window.chrome", "'object'"),
        ("typeof window.chrome.runtime", "'object'"),
        ("navigator.plugins.length > 0", "true"),
        (
            &format!("navigator.platform === '{}'", fp.platform),
            "true",
        ),
        (
            &format!(
                "navigator.hardwareConcurrency === {}",
                fp.hardware_concurrency
            ),
            "true",
        ),
        (
            &format!("navigator.deviceMemory === {}", fp.device_memory),
            "true",
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;
    for (expr, expected) in &checks {
        let result = page
            .evaluate(format!("String({expr})"))
            .await
            .map(|v: String| v)
            .unwrap_or_else(|_| "ERROR".to_string());
        if result == *expected {
            println!("   ✅ {expr} = {result}");
            passed += 1;
        } else {
            println!("   ❌ {expr} = {result} (expected {expected})");
            failed += 1;
        }
    }

    println!("🔒 Stealth results: {passed} passed, {failed} failed out of {} checks", checks.len());

    // Save stealth screenshot
    use onecrawl_cdp::screenshot;
    if let Ok(bytes) = screenshot::screenshot_full(&page).await {
        let dir = report_dir();
        fs::write(dir.join("chromiumoxide_stealth.png"), &bytes).unwrap();
        println!("   📸 Stealth screenshot saved");
    }
}

// ─── Playwright-rs Benchmarks (only with feature flag) ───

#[cfg(feature = "onecrawl-cdp/playwright")]
mod playwright_tests {
    use super::*;
    use onecrawl_cdp::playwright_backend::{BrowserEngine, PlaywrightSession};

    #[tokio::test]
    async fn bench_playwright_launch() {
        let start = Instant::now();
        let session = PlaywrightSession::launch(BrowserEngine::Chromium, true).await;
        let elapsed = start.elapsed();

        match session {
            Ok(session) => {
                println!(
                    "✅ playwright-rs launch (Chromium): {:.0}ms",
                    elapsed.as_millis()
                );
                let dir = report_dir();
                fs::write(
                    dir.join("playwright_launch_ms.txt"),
                    format!("{}", elapsed.as_millis()),
                )
                .unwrap();
                let _ = session.close().await;
            }
            Err(e) => println!("❌ playwright-rs launch failed: {e}"),
        }
    }

    #[tokio::test]
    async fn bench_playwright_navigate_screenshot() {
        let session = match PlaywrightSession::launch(BrowserEngine::Chromium, true).await {
            Ok(s) => s,
            Err(e) => {
                println!("⚠️  skipping playwright test: {e}");
                return;
            }
        };

        let nav_start = Instant::now();
        let nav_result = session.navigate("https://example.com").await;
        let nav_elapsed = nav_start.elapsed();

        match nav_result {
            Ok(_) => println!(
                "✅ playwright-rs navigate: {:.0}ms",
                nav_elapsed.as_millis()
            ),
            Err(e) => {
                println!("❌ playwright-rs navigate failed: {e}");
                return;
            }
        }

        let ss_start = Instant::now();
        let ss_result = session.screenshot().await;
        let ss_elapsed = ss_start.elapsed();

        match ss_result {
            Ok(bytes) => {
                println!(
                    "✅ playwright-rs screenshot: {:.0}ms ({} bytes)",
                    ss_elapsed.as_millis(),
                    bytes.len()
                );
                let dir = report_dir();
                fs::write(dir.join("playwright_example.png"), &bytes).unwrap();
                fs::write(
                    dir.join("playwright_nav_ms.txt"),
                    format!("{}", nav_elapsed.as_millis()),
                )
                .unwrap();
                fs::write(
                    dir.join("playwright_screenshot_ms.txt"),
                    format!("{}", ss_elapsed.as_millis()),
                )
                .unwrap();
            }
            Err(e) => println!("❌ playwright-rs screenshot failed: {e}"),
        }

        let content = session.content().await.unwrap_or_default();
        println!("   Content length: {} chars", content.len());

        let _ = session.close().await;
    }

    #[tokio::test]
    async fn bench_playwright_firefox() {
        let start = Instant::now();
        let session = PlaywrightSession::launch(BrowserEngine::Firefox, true).await;
        let elapsed = start.elapsed();

        match session {
            Ok(session) => {
                println!(
                    "✅ playwright-rs Firefox launch: {:.0}ms",
                    elapsed.as_millis()
                );
                let _ = session.navigate("https://example.com").await;
                if let Ok(bytes) = session.screenshot().await {
                    let dir = report_dir();
                    fs::write(dir.join("playwright_firefox.png"), &bytes).unwrap();
                }
                let _ = session.close().await;
            }
            Err(e) => println!("⚠️  Firefox not available: {e}"),
        }
    }

    #[tokio::test]
    async fn bench_playwright_webkit() {
        let start = Instant::now();
        let session = PlaywrightSession::launch(BrowserEngine::Webkit, true).await;
        let elapsed = start.elapsed();

        match session {
            Ok(session) => {
                println!(
                    "✅ playwright-rs WebKit launch: {:.0}ms",
                    elapsed.as_millis()
                );
                let _ = session.navigate("https://example.com").await;
                if let Ok(bytes) = session.screenshot().await {
                    let dir = report_dir();
                    fs::write(dir.join("playwright_webkit.png"), &bytes).unwrap();
                }
                let _ = session.close().await;
            }
            Err(e) => println!("⚠️  WebKit not available: {e}"),
        }
    }
}
