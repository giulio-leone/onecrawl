//! Performance Monitor — Core Web Vitals, performance budgets,
//! regression detection, and real-time metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core Web Vitals metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreWebVitals {
    pub lcp_ms: Option<f64>,
    pub fid_ms: Option<f64>,
    pub cls: Option<f64>,
    pub fcp_ms: Option<f64>,
    pub ttfb_ms: Option<f64>,
    pub inp_ms: Option<f64>,
}

/// Performance metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfSnapshot {
    pub url: String,
    pub timestamp: u64,
    pub vitals: CoreWebVitals,
    pub navigation_timing: NavigationTiming,
    pub resource_count: ResourceCount,
    pub memory: Option<MemoryInfo>,
    pub js_heap_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NavigationTiming {
    pub dns_ms: f64,
    pub tcp_ms: f64,
    pub tls_ms: f64,
    pub ttfb_ms: f64,
    pub download_ms: f64,
    pub dom_interactive_ms: f64,
    pub dom_complete_ms: f64,
    pub load_event_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceCount {
    pub total: usize,
    pub scripts: usize,
    pub stylesheets: usize,
    pub images: usize,
    pub fonts: usize,
    pub xhr: usize,
    pub other: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub used_js_heap: u64,
    pub total_js_heap: u64,
    pub js_heap_limit: u64,
}

/// Performance budget definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfBudget {
    pub name: String,
    pub lcp_ms: Option<f64>,
    pub fid_ms: Option<f64>,
    pub cls: Option<f64>,
    pub fcp_ms: Option<f64>,
    pub ttfb_ms: Option<f64>,
    pub total_load_ms: Option<f64>,
    pub max_requests: Option<usize>,
    pub max_transfer_bytes: Option<u64>,
    pub max_scripts: Option<usize>,
}

/// Budget check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetResult {
    pub budget_name: String,
    pub passed: bool,
    pub checks: Vec<BudgetCheck>,
    pub violations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCheck {
    pub metric: String,
    pub budget: f64,
    pub actual: f64,
    pub passed: bool,
    pub delta: f64,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Ok,
    Warning,
    Critical,
}

/// Check a snapshot against a budget.
pub fn check_budget(snapshot: &PerfSnapshot, budget: &PerfBudget) -> BudgetResult {
    let mut checks = Vec::new();

    if let (Some(budget_val), Some(actual)) = (budget.lcp_ms, snapshot.vitals.lcp_ms) {
        checks.push(metric_check("LCP", budget_val, actual, 1.5));
    }
    if let (Some(budget_val), Some(actual)) = (budget.fid_ms, snapshot.vitals.fid_ms) {
        checks.push(metric_check("FID", budget_val, actual, 1.3));
    }
    if let (Some(budget_val), Some(actual)) = (budget.cls, snapshot.vitals.cls) {
        checks.push(metric_check("CLS", budget_val, actual, 1.5));
    }
    if let (Some(budget_val), Some(actual)) = (budget.fcp_ms, snapshot.vitals.fcp_ms) {
        checks.push(metric_check("FCP", budget_val, actual, 1.5));
    }
    if let (Some(budget_val), Some(actual)) = (budget.ttfb_ms, snapshot.vitals.ttfb_ms) {
        checks.push(metric_check("TTFB", budget_val, actual, 1.3));
    }
    if let Some(budget_val) = budget.total_load_ms {
        checks.push(metric_check("Total Load", budget_val, snapshot.navigation_timing.total_ms, 1.5));
    }
    if let Some(max) = budget.max_requests {
        checks.push(metric_check("Requests", max as f64, snapshot.resource_count.total as f64, 1.2));
    }
    if let Some(max) = budget.max_transfer_bytes {
        checks.push(metric_check("Transfer Size", max as f64, snapshot.resource_count.total_bytes as f64, 1.2));
    }
    if let Some(max) = budget.max_scripts {
        checks.push(metric_check("Scripts", max as f64, snapshot.resource_count.scripts as f64, 1.3));
    }

    let violations = checks.iter().filter(|c| !c.passed).count();
    BudgetResult {
        budget_name: budget.name.clone(),
        passed: violations == 0,
        checks,
        violations,
    }
}

fn metric_check(name: &str, budget: f64, actual: f64, warning_factor: f64) -> BudgetCheck {
    let delta = actual - budget;
    let passed = actual <= budget;
    let severity = if actual <= budget {
        Severity::Ok
    } else if actual <= budget * warning_factor {
        Severity::Warning
    } else {
        Severity::Critical
    };

    BudgetCheck {
        metric: name.to_string(),
        budget,
        actual,
        passed,
        delta,
        severity,
    }
}

/// Detect regressions between two snapshots.
pub fn detect_regressions(baseline: &PerfSnapshot, current: &PerfSnapshot, threshold_pct: f64) -> Vec<Regression> {
    let mut regressions = Vec::new();

    if let (Some(b), Some(c)) = (baseline.vitals.lcp_ms, current.vitals.lcp_ms) {
        if let Some(r) = check_regression("LCP", b, c, threshold_pct) { regressions.push(r); }
    }
    if let (Some(b), Some(c)) = (baseline.vitals.fcp_ms, current.vitals.fcp_ms) {
        if let Some(r) = check_regression("FCP", b, c, threshold_pct) { regressions.push(r); }
    }
    if let (Some(b), Some(c)) = (baseline.vitals.cls, current.vitals.cls) {
        if let Some(r) = check_regression("CLS", b, c, threshold_pct) { regressions.push(r); }
    }
    if let (Some(b), Some(c)) = (baseline.vitals.ttfb_ms, current.vitals.ttfb_ms) {
        if let Some(r) = check_regression("TTFB", b, c, threshold_pct) { regressions.push(r); }
    }

    let b_total = baseline.navigation_timing.total_ms;
    let c_total = current.navigation_timing.total_ms;
    if b_total > 0.0 {
        if let Some(r) = check_regression("Total Load", b_total, c_total, threshold_pct) { regressions.push(r); }
    }

    regressions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub delta_pct: f64,
    pub severity: Severity,
}

fn check_regression(name: &str, baseline: f64, current: f64, threshold_pct: f64) -> Option<Regression> {
    if baseline <= 0.0 { return None; }
    let delta = current - baseline;
    let delta_pct = (delta / baseline) * 100.0;

    if delta_pct > threshold_pct {
        let severity = if delta_pct > threshold_pct * 2.0 { Severity::Critical } else { Severity::Warning };
        Some(Regression { metric: name.to_string(), baseline, current, delta, delta_pct, severity })
    } else {
        None
    }
}

/// Rate a Core Web Vitals score based on Google's thresholds.
pub fn rate_vitals(vitals: &CoreWebVitals) -> HashMap<String, String> {
    let mut ratings = HashMap::new();

    if let Some(lcp) = vitals.lcp_ms {
        ratings.insert("LCP".into(), if lcp <= 2500.0 { "good" } else if lcp <= 4000.0 { "needs_improvement" } else { "poor" }.into());
    }
    if let Some(fid) = vitals.fid_ms {
        ratings.insert("FID".into(), if fid <= 100.0 { "good" } else if fid <= 300.0 { "needs_improvement" } else { "poor" }.into());
    }
    if let Some(cls) = vitals.cls {
        ratings.insert("CLS".into(), if cls <= 0.1 { "good" } else if cls <= 0.25 { "needs_improvement" } else { "poor" }.into());
    }
    if let Some(fcp) = vitals.fcp_ms {
        ratings.insert("FCP".into(), if fcp <= 1800.0 { "good" } else if fcp <= 3000.0 { "needs_improvement" } else { "poor" }.into());
    }
    if let Some(ttfb) = vitals.ttfb_ms {
        ratings.insert("TTFB".into(), if ttfb <= 800.0 { "good" } else if ttfb <= 1800.0 { "needs_improvement" } else { "poor" }.into());
    }
    if let Some(inp) = vitals.inp_ms {
        ratings.insert("INP".into(), if inp <= 200.0 { "good" } else if inp <= 500.0 { "needs_improvement" } else { "poor" }.into());
    }

    ratings
}

/// Generate a JS snippet to collect performance metrics from the browser.
pub fn metrics_collection_js() -> &'static str {
    r#"(() => {
    const nav = performance.getEntriesByType('navigation')[0] || {};
    const paint = performance.getEntriesByType('paint') || [];
    const resources = performance.getEntriesByType('resource') || [];

    const fcp = paint.find(e => e.name === 'first-contentful-paint');

    let lcp = null;
    try {
        const lcpEntries = performance.getEntriesByType('largest-contentful-paint');
        if (lcpEntries.length > 0) lcp = lcpEntries[lcpEntries.length - 1].startTime;
    } catch(e) {}

    let cls = 0;
    try {
        const layoutShifts = performance.getEntriesByType('layout-shift');
        layoutShifts.forEach(e => { if (!e.hadRecentInput) cls += e.value; });
    } catch(e) {}

    const resources_count = { total: resources.length, scripts: 0, stylesheets: 0, images: 0, fonts: 0, xhr: 0, other: 0, total_bytes: 0 };
    resources.forEach(r => {
        resources_count.total_bytes += r.transferSize || 0;
        switch(r.initiatorType) {
            case 'script': resources_count.scripts++; break;
            case 'link': case 'css': resources_count.stylesheets++; break;
            case 'img': resources_count.images++; break;
            case 'font': resources_count.fonts++; break;
            case 'xmlhttprequest': case 'fetch': resources_count.xhr++; break;
            default: resources_count.other++;
        }
    });

    let memory = null;
    if (performance.memory) {
        memory = {
            used_js_heap: performance.memory.usedJSHeapSize,
            total_js_heap: performance.memory.totalJSHeapSize,
            js_heap_limit: performance.memory.jsHeapSizeLimit,
        };
    }

    return {
        vitals: {
            lcp_ms: lcp,
            fid_ms: null,
            cls: cls > 0 ? cls : null,
            fcp_ms: fcp ? fcp.startTime : null,
            ttfb_ms: nav.responseStart ? nav.responseStart - nav.requestStart : null,
            inp_ms: null,
        },
        navigation_timing: {
            dns_ms: (nav.domainLookupEnd || 0) - (nav.domainLookupStart || 0),
            tcp_ms: (nav.connectEnd || 0) - (nav.connectStart || 0),
            tls_ms: nav.secureConnectionStart ? (nav.connectEnd || 0) - nav.secureConnectionStart : 0,
            ttfb_ms: (nav.responseStart || 0) - (nav.requestStart || 0),
            download_ms: (nav.responseEnd || 0) - (nav.responseStart || 0),
            dom_interactive_ms: nav.domInteractive || 0,
            dom_complete_ms: nav.domComplete || 0,
            load_event_ms: nav.loadEventEnd || 0,
            total_ms: nav.loadEventEnd || nav.domComplete || 0,
        },
        resource_count: resources_count,
        memory: memory,
    };
})()"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> PerfSnapshot {
        PerfSnapshot {
            url: "https://example.com".into(),
            timestamp: 1700000000,
            vitals: CoreWebVitals {
                lcp_ms: Some(2000.0),
                fid_ms: Some(50.0),
                cls: Some(0.05),
                fcp_ms: Some(1500.0),
                ttfb_ms: Some(500.0),
                inp_ms: Some(150.0),
            },
            navigation_timing: NavigationTiming {
                dns_ms: 20.0, tcp_ms: 30.0, tls_ms: 40.0,
                ttfb_ms: 500.0, download_ms: 100.0,
                dom_interactive_ms: 1200.0, dom_complete_ms: 2000.0,
                load_event_ms: 2100.0, total_ms: 2100.0,
            },
            resource_count: ResourceCount {
                total: 25, scripts: 8, stylesheets: 3, images: 10,
                fonts: 2, xhr: 2, other: 0, total_bytes: 512000,
            },
            memory: None,
            js_heap_size: None,
        }
    }

    #[test]
    fn rate_good_vitals() {
        let vitals = CoreWebVitals {
            lcp_ms: Some(2000.0), fid_ms: Some(50.0), cls: Some(0.05),
            fcp_ms: Some(1500.0), ttfb_ms: Some(500.0), inp_ms: Some(150.0),
        };
        let ratings = rate_vitals(&vitals);
        assert_eq!(ratings["LCP"], "good");
        assert_eq!(ratings["FID"], "good");
        assert_eq!(ratings["CLS"], "good");
    }

    #[test]
    fn rate_poor_vitals() {
        let vitals = CoreWebVitals {
            lcp_ms: Some(5000.0), fid_ms: Some(500.0), cls: Some(0.5),
            fcp_ms: Some(4000.0), ttfb_ms: Some(2000.0), inp_ms: Some(600.0),
        };
        let ratings = rate_vitals(&vitals);
        assert_eq!(ratings["LCP"], "poor");
        assert_eq!(ratings["FID"], "poor");
        assert_eq!(ratings["CLS"], "poor");
    }

    #[test]
    fn budget_all_pass() {
        let snap = sample_snapshot();
        let budget = PerfBudget {
            name: "standard".into(),
            lcp_ms: Some(3000.0), fid_ms: Some(100.0), cls: Some(0.1),
            fcp_ms: Some(2000.0), ttfb_ms: Some(800.0),
            total_load_ms: Some(3000.0), max_requests: Some(50),
            max_transfer_bytes: Some(1_000_000), max_scripts: Some(15),
        };
        let result = check_budget(&snap, &budget);
        assert!(result.passed);
        assert_eq!(result.violations, 0);
    }

    #[test]
    fn budget_violations() {
        let snap = sample_snapshot();
        let budget = PerfBudget {
            name: "strict".into(),
            lcp_ms: Some(1000.0), fid_ms: None, cls: Some(0.01),
            fcp_ms: None, ttfb_ms: None,
            total_load_ms: None, max_requests: Some(10),
            max_transfer_bytes: None, max_scripts: Some(5),
        };
        let result = check_budget(&snap, &budget);
        assert!(!result.passed);
        assert!(result.violations >= 3);
    }

    #[test]
    fn severity_classification() {
        let check = metric_check("LCP", 2500.0, 3000.0, 1.5);
        assert!(!check.passed);
        assert_eq!(check.severity, Severity::Warning);

        let check2 = metric_check("LCP", 2500.0, 5000.0, 1.5);
        assert_eq!(check2.severity, Severity::Critical);
    }

    #[test]
    fn no_regression() {
        let baseline = sample_snapshot();
        let current = sample_snapshot();
        let regressions = detect_regressions(&baseline, &current, 10.0);
        assert!(regressions.is_empty());
    }

    #[test]
    fn detect_regression_lcp() {
        let baseline = sample_snapshot();
        let mut current = sample_snapshot();
        current.vitals.lcp_ms = Some(3000.0);
        let regressions = detect_regressions(&baseline, &current, 10.0);
        assert!(!regressions.is_empty());
        assert_eq!(regressions[0].metric, "LCP");
        assert!(regressions[0].delta_pct > 40.0);
    }

    #[test]
    fn detect_critical_regression() {
        let baseline = sample_snapshot();
        let mut current = sample_snapshot();
        current.vitals.lcp_ms = Some(5000.0);
        let regressions = detect_regressions(&baseline, &current, 10.0);
        assert_eq!(regressions[0].severity, Severity::Critical);
    }

    #[test]
    fn metrics_js_not_empty() {
        let js = metrics_collection_js();
        assert!(!js.is_empty());
        assert!(js.contains("performance"));
        assert!(js.contains("largest-contentful-paint"));
    }

    #[test]
    fn budget_check_delta() {
        let check = metric_check("Test", 100.0, 150.0, 1.5);
        assert_eq!(check.delta, 50.0);
        assert_eq!(check.budget, 100.0);
        assert_eq!(check.actual, 150.0);
    }
}
