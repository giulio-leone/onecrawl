//! OneCrawl E2E Benchmark Suite
//! Tests chromiumoxide vs playwright-rs, stealth patches, crypto, parser, storage

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn report_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .join("reports").join("benchmark");
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ─── Chromiumoxide Benchmark ───

async fn run_chromiumoxide_benchmarks(results: &mut Vec<(String, String)>) {
    use onecrawl_cdp::browser::BrowserSession;
    use onecrawl_cdp::page::evaluate_js;
    use onecrawl_cdp::screenshot;
    use onecrawl_cdp::stealth::fingerprint::generate_fingerprint;
    use onecrawl_cdp::stealth::scripts::get_stealth_init_script;

    println!("\n============================================================");
    println!("  CHROMIUMOXIDE (NATIVE CDP) BENCHMARK");
    println!("============================================================\n");

    let start = Instant::now();
    let session = match BrowserSession::launch_headless().await {
        Ok(s) => {
            let ms = start.elapsed().as_millis();
            println!("  ✅ Launch: {ms}ms");
            results.push(("chromiumoxide_launch_ms".into(), format!("{ms}")));
            s
        }
        Err(e) => {
            println!("  ❌ Launch failed: {e}");
            println!("     (Chrome/Chromium must be installed)");
            return;
        }
    };

    let page = match session.new_page("about:blank").await {
        Ok(p) => p,
        Err(e) => { println!("  ❌ new_page: {e}"); return; }
    };

    // Navigate
    let nav_start = Instant::now();
    match onecrawl_cdp::navigation::goto(&page, "https://example.com").await {
        Ok(_) => {
            let ms = nav_start.elapsed().as_millis();
            println!("  ✅ Navigate to example.com: {ms}ms");
            results.push(("chromiumoxide_nav_ms".into(), format!("{ms}")));
        }
        Err(e) => { println!("  ❌ Navigate: {e}"); return; }
    }

    // Title
    match evaluate_js(&page, "document.title").await {
        Ok(v) => println!("  ✅ Page title: {v}"),
        Err(e) => println!("  ⚠️  Eval: {e}"),
    }

    // Screenshot
    let ss_start = Instant::now();
    match screenshot::screenshot_full(&page).await {
        Ok(bytes) => {
            let ms = ss_start.elapsed().as_millis();
            println!("  ✅ Screenshot: {ms}ms ({} bytes)", bytes.len());
            results.push(("chromiumoxide_screenshot_ms".into(), format!("{ms}")));
            results.push(("chromiumoxide_screenshot_bytes".into(), format!("{}", bytes.len())));
            fs::write(report_dir().join("chromiumoxide_example.png"), &bytes).unwrap();
        }
        Err(e) => println!("  ❌ Screenshot: {e}"),
    }

    // ── Stealth ──
    println!("\n-- Stealth Patches --");
    let fp = generate_fingerprint();
    let script = get_stealth_init_script(&fp);
    println!("  Fingerprint: platform={}, hw={}, mem={}GB", fp.platform, fp.hardware_concurrency, fp.device_memory);
    println!("  Script: {} chars", script.len());

    let inject_start = Instant::now();
    let inject_js = format!("(function() {{ {script}; return 'injected'; }})()");
    match evaluate_js(&page, &inject_js).await {
        Ok(v) => {
            let ms = inject_start.elapsed().as_millis();
            println!("  ✅ Injection: {ms}ms (result: {v})");
            results.push(("stealth_injection_ms".into(), format!("{ms}")));
        }
        Err(e) => println!("  ❌ Injection: {e}"),
    }

    // Verify stealth patches
    let checks: Vec<(&str, String)> = vec![
        ("String(navigator.webdriver)", "false".into()),
        ("String(typeof window.chrome.runtime)", "object".into()),
        ("String(navigator.plugins.length > 0)", "true".into()),
        ("String(navigator.platform)", fp.platform.clone()),
        ("String(navigator.hardwareConcurrency)", format!("{}", fp.hardware_concurrency)),
        ("String(navigator.deviceMemory)", format!("{}", fp.device_memory)),
    ];

    let mut stealth_pass = 0;
    let total = checks.len();
    for (expr, expected) in &checks {
        match evaluate_js(&page, expr).await {
            Ok(v) => {
                let val = v.as_str().unwrap_or("").to_string();
                if val == *expected {
                    println!("    ✅ {expr} = {val}");
                    stealth_pass += 1;
                } else {
                    println!("    ❌ {expr} = {val} (expected {expected})");
                }
            }
            Err(e) => println!("    ❌ {expr}: {e}"),
        }
    }
    results.push(("stealth_checks_passed".into(), format!("{stealth_pass}/{total}")));

    // Stealth screenshot
    if let Ok(bytes) = screenshot::screenshot_full(&page).await {
        fs::write(report_dir().join("chromiumoxide_stealth.png"), &bytes).unwrap();
        println!("  📸 Stealth screenshot saved");
    }
}

// ─── Playwright-rs Benchmark ───

#[cfg(feature = "playwright")]
async fn run_playwright_benchmarks(results: &mut Vec<(String, String)>) {
    use onecrawl_cdp::playwright_backend::{BrowserEngine, PlaywrightSession};

    println!("\n============================================================");
    println!("  PLAYWRIGHT-RS BENCHMARK");
    println!("============================================================\n");

    for (engine, name) in [
        (BrowserEngine::Chromium, "Chromium"),
        (BrowserEngine::Firefox, "Firefox"),
        (BrowserEngine::Webkit, "WebKit"),
    ] {
        println!("-- {name} --");
        let start = Instant::now();
        match PlaywrightSession::launch(engine, true).await {
            Ok(session) => {
                let launch_ms = start.elapsed().as_millis();
                println!("  ✅ Launch: {launch_ms}ms");
                results.push((format!("playwright_{}_launch_ms", name.to_lowercase()), format!("{launch_ms}")));

                let nav_start = Instant::now();
                if session.navigate("https://example.com").await.is_ok() {
                    let nav_ms = nav_start.elapsed().as_millis();
                    println!("  ✅ Navigate: {nav_ms}ms");
                    results.push((format!("playwright_{}_nav_ms", name.to_lowercase()), format!("{nav_ms}")));
                }

                let ss_start = Instant::now();
                if let Ok(bytes) = session.screenshot().await {
                    let ss_ms = ss_start.elapsed().as_millis();
                    println!("  ✅ Screenshot: {ss_ms}ms ({} bytes)", bytes.len());
                    results.push((format!("playwright_{}_screenshot_ms", name.to_lowercase()), format!("{ss_ms}")));
                    fs::write(report_dir().join(format!("playwright_{}.png", name.to_lowercase())), &bytes).unwrap();
                }

                if let Ok(html) = session.content().await {
                    println!("  ✅ Content: {} chars", html.len());
                }

                let _ = session.close().await;
            }
            Err(e) => println!("  ⚠️  {name} not available: {e}"),
        }
        println!();
    }
}

#[cfg(not(feature = "playwright"))]
async fn run_playwright_benchmarks(_results: &mut Vec<(String, String)>) {
    println!("\n⚠️  playwright feature not enabled — skipping playwright-rs benchmarks");
    println!("   Run with: cargo run -p onecrawl-benchmark --features playwright");
}

// ─── Crypto Benchmark ───

async fn run_crypto_benchmarks(results: &mut Vec<(String, String)>) {
    use onecrawl_crypto::{encrypt, decrypt, generate_pkce_challenge, generate_totp};
    use onecrawl_core::types::TotpConfig;

    println!("\n============================================================");
    println!("  CRYPTO BENCHMARKS");
    println!("============================================================\n");

    let data = b"OneCrawl benchmark data for encryption test - realistic payload";
    let passphrase = "benchmark-passphrase-strong";

    // Encrypt
    let iterations = 1000u64;
    let start = Instant::now();
    let mut last_payload = None;
    for _ in 0..iterations {
        last_payload = Some(encrypt(data, passphrase).unwrap());
    }
    let total_ms = start.elapsed().as_millis();
    let avg_us = start.elapsed().as_micros() / iterations as u128;
    println!("  ✅ Encrypt ({iterations}x): {total_ms}ms total, avg {avg_us}μs");
    results.push(("crypto_encrypt_avg_us".into(), format!("{avg_us}")));

    // Decrypt
    let payload = last_payload.unwrap();
    let dec_start = Instant::now();
    for _ in 0..iterations {
        let _ = decrypt(&payload, passphrase).unwrap();
    }
    let dec_total_ms = dec_start.elapsed().as_millis();
    let dec_avg_us = dec_start.elapsed().as_micros() / iterations as u128;
    println!("  ✅ Decrypt ({iterations}x): {dec_total_ms}ms total, avg {dec_avg_us}μs");
    results.push(("crypto_decrypt_avg_us".into(), format!("{dec_avg_us}")));

    // PKCE
    let pkce_start = Instant::now();
    let challenge = generate_pkce_challenge().unwrap();
    let pkce_us = pkce_start.elapsed().as_micros();
    println!("  ✅ PKCE: {pkce_us}μs (verifier={}, challenge={})", challenge.code_verifier.len(), challenge.code_challenge.len());
    results.push(("crypto_pkce_us".into(), format!("{pkce_us}")));

    // TOTP
    let totp_config = TotpConfig {
        secret: "JBSWY3DPEHPK3PXP".into(),
        digits: 6,
        period: 30,
        algorithm: onecrawl_core::types::TotpAlgorithm::Sha1,
    };
    let totp_start = Instant::now();
    let code = generate_totp(&totp_config).unwrap();
    let totp_us = totp_start.elapsed().as_micros();
    println!("  ✅ TOTP: {totp_us}μs (code={code})");
    results.push(("crypto_totp_us".into(), format!("{totp_us}")));
}

// ─── Parser Benchmark ───

async fn run_parser_benchmarks(results: &mut Vec<(String, String)>) {
    use onecrawl_parser::{get_accessibility_tree, query_selector, extract_text};
    use onecrawl_parser::extract::extract_links;

    println!("\n============================================================");
    println!("  PARSER BENCHMARKS");
    println!("============================================================\n");

    let html = r#"<!DOCTYPE html><html><head><title>Benchmark</title></head><body>
        <nav><a href="/home">Home</a><a href="/about">About</a><a href="/contact">Contact</a></nav>
        <main><h1>OneCrawl</h1><p>Testing parser performance.</p>
        <ul><li>Item 1</li><li>Item 2</li><li>Item 3</li><li>Item 4</li><li>Item 5</li></ul>
        <div class="card"><h2>Card</h2><p>Content</p><a href="/details">Details</a></div>
        <table><tr><th>Name</th><th>Score</th></tr><tr><td>Alice</td><td>95</td></tr></table>
        </main><footer><p>© 2025</p></footer></body></html>"#;

    // A11y tree
    let start = Instant::now();
    let tree = get_accessibility_tree(html).unwrap();
    let us = start.elapsed().as_micros();
    println!("  ✅ Accessibility tree: {us}μs (role={})", tree.role);
    results.push(("parser_a11y_us".into(), format!("{us}")));

    // Query selector
    let qs_start = Instant::now();
    let items = query_selector(html, "li").unwrap();
    let qs_us = qs_start.elapsed().as_micros();
    println!("  ✅ Query selector (li): {qs_us}μs ({} matches)", items.len());
    results.push(("parser_query_us".into(), format!("{qs_us}")));

    // Extract text
    let text_start = Instant::now();
    let texts = extract_text(html, "p").unwrap();
    let text_us = text_start.elapsed().as_micros();
    println!("  ✅ Extract text (p): {text_us}μs ({} results)", texts.len());
    results.push(("parser_text_us".into(), format!("{text_us}")));

    // Extract links
    let links_start = Instant::now();
    let links = extract_links(html).unwrap();
    let links_us = links_start.elapsed().as_micros();
    println!("  ✅ Extract links: {links_us}μs ({} links)", links.len());
    results.push(("parser_links_us".into(), format!("{links_us}")));
}

// ─── Storage Benchmark ───

async fn run_storage_benchmarks(results: &mut Vec<(String, String)>) {
    use onecrawl_storage::EncryptedStore;

    println!("\n============================================================");
    println!("  STORAGE BENCHMARKS");
    println!("============================================================\n");

    let dir = tempfile::tempdir().unwrap();
    let store = EncryptedStore::open(dir.path(), "benchmark-passphrase").unwrap();

    let iterations = 100u64;

    // Write
    let start = Instant::now();
    for i in 0..iterations {
        store.set(&format!("bench-key-{i}"), format!("value-{i}-{}", "x".repeat(100)).as_bytes()).unwrap();
    }
    let write_ms = start.elapsed().as_millis();
    let avg_write_us = start.elapsed().as_micros() / iterations as u128;
    println!("  ✅ Write ({iterations}x): {write_ms}ms (avg {avg_write_us}μs)");
    results.push(("storage_write_avg_us".into(), format!("{avg_write_us}")));

    // Read
    let read_start = Instant::now();
    for i in 0..iterations {
        let _ = store.get(&format!("bench-key-{i}")).unwrap();
    }
    let read_ms = read_start.elapsed().as_millis();
    let avg_read_us = read_start.elapsed().as_micros() / iterations as u128;
    println!("  ✅ Read ({iterations}x): {read_ms}ms (avg {avg_read_us}μs)");
    results.push(("storage_read_avg_us".into(), format!("{avg_read_us}")));

    // List
    let list_start = Instant::now();
    let keys = store.list("bench-").unwrap();
    let list_us = list_start.elapsed().as_micros();
    println!("  ✅ List: {list_us}μs ({} keys)", keys.len());
    results.push(("storage_list_us".into(), format!("{list_us}")));
}

fn generate_report(results: &[(String, String)]) {
    let dir = report_dir();
    let date = {
        let out = std::process::Command::new("date").arg("+%Y-%m-%dT%H:%M:%S%z").output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };

    let mut md = String::new();
    md.push_str("# OneCrawl E2E Benchmark Report\n\n");
    md.push_str(&format!("**Date:** {date}\n"));
    md.push_str(&format!("**Platform:** {} {}\n\n", std::env::consts::OS, std::env::consts::ARCH));

    md.push_str("## Results\n\n");
    md.push_str("| Metric | Value |\n|--------|-------|\n");
    for (k, v) in results {
        md.push_str(&format!("| {k} | {v} |\n"));
    }

    md.push_str("\n## Screenshots\n\n");
    for (file, label) in [
        ("chromiumoxide_example.png", "Chromiumoxide — example.com"),
        ("chromiumoxide_stealth.png", "Chromiumoxide — Stealth Patched"),
        ("playwright_chromium.png", "Playwright-rs — Chromium"),
        ("playwright_firefox.png", "Playwright-rs — Firefox"),
        ("playwright_webkit.png", "Playwright-rs — WebKit"),
    ] {
        if dir.join(file).exists() {
            md.push_str(&format!("### {label}\n![{label}]({file})\n\n"));
        }
    }

    fs::write(dir.join("REPORT.md"), &md).unwrap();
    println!("\n📄 Report: reports/benchmark/REPORT.md");
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       ONECRAWL E2E BENCHMARK SUITE                     ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let mut results = Vec::new();

    run_chromiumoxide_benchmarks(&mut results).await;
    run_playwright_benchmarks(&mut results).await;
    run_crypto_benchmarks(&mut results).await;
    run_parser_benchmarks(&mut results).await;
    run_storage_benchmarks(&mut results).await;

    generate_report(&results);

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║       BENCHMARK COMPLETE                               ║");
    println!("╚══════════════════════════════════════════════════════════╝");
}
