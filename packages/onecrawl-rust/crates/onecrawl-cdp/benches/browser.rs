//! Performance benchmarks for onecrawl-cdp browser operations.
//!
//! These benchmarks require a running Chromium instance and are designed to be
//! run with `cargo bench -p onecrawl-cdp` once Criterion is added as a
//! dev-dependency. The file structure is ready; just uncomment and add:
//!
//! ```toml
//! [dev-dependencies]
//! criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
//!
//! [[bench]]
//! name = "browser"
//! harness = false
//! ```

// NOTE: Criterion is intentionally NOT added as a dependency yet because these
// benchmarks require a live browser. The structure below is ready to activate.

/*
use criterion::{criterion_group, criterion_main, Criterion};

async fn bench_navigate(page: &chromiumoxide::Page) {
    onecrawl_cdp::navigation::goto(page, "about:blank").await.unwrap();
    onecrawl_cdp::navigation::wait_ms(10).await;
}

async fn bench_screenshot(page: &chromiumoxide::Page) {
    onecrawl_cdp::screenshot::screenshot_viewport(page).await.unwrap();
}

fn navigation_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Requires a running browser session
    // let session = rt.block_on(onecrawl_cdp::BrowserSession::launch_headless()).unwrap();
    // let page = rt.block_on(session.new_page("about:blank")).unwrap();

    c.bench_function("navigate_about_blank", |b| {
        b.to_async(&rt).iter(|| async {
            // bench_navigate(&page).await;
        });
    });
}

fn screenshot_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("screenshot_viewport", |b| {
        b.to_async(&rt).iter(|| async {
            // bench_screenshot(&page).await;
        });
    });

    c.bench_function("screenshot_full_page", |b| {
        b.to_async(&rt).iter(|| async {
            // onecrawl_cdp::screenshot::screenshot_full(&page).await.unwrap();
        });
    });
}

fn parser_benchmark(c: &mut Criterion) {
    let html = "<html><body><h1>Hello</h1><p>World</p></body></html>";
    c.bench_function("html_parse_small", |b| {
        b.iter(|| {
            // Benchmark HTML parsing throughput (uses onecrawl-parser crate)
            // onecrawl_parser::parse(html).unwrap();
        });
    });
}

criterion_group!(benches, navigation_benchmark, screenshot_benchmark, parser_benchmark);
criterion_main!(benches);
*/

fn main() {
    eprintln!(
        "Benchmarks are stubbed out. Add criterion as a dev-dependency and \
         uncomment the benchmark code to enable."
    );
}
