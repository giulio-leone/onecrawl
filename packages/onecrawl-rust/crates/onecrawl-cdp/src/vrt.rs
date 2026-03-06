//! Visual Regression Testing — baseline capture, pixel-perfect comparison,
//! threshold-based approval, test suite runner, CI integration.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A VRT test case definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrtTestCase {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub full_page: bool,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default)]
    pub viewport: Option<VrtViewport>,
    #[serde(default)]
    pub wait_for: Option<String>,
    #[serde(default)]
    pub hide_selectors: Vec<String>,
    #[serde(default)]
    pub delay_ms: u64,
}

fn default_threshold() -> f64 {
    0.1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrtViewport {
    pub width: u32,
    pub height: u32,
}

/// A VRT test suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrtSuite {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_baseline_dir")]
    pub baseline_dir: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_diff_dir")]
    pub diff_dir: String,
    #[serde(default = "default_global_threshold")]
    pub threshold: f64,
    pub tests: Vec<VrtTestCase>,
}

fn default_baseline_dir() -> String {
    ".vrt/baselines".into()
}
fn default_output_dir() -> String {
    ".vrt/current".into()
}
fn default_diff_dir() -> String {
    ".vrt/diffs".into()
}
fn default_global_threshold() -> f64 {
    0.1
}

/// Result of a single VRT comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrtComparisonResult {
    pub test_name: String,
    pub status: VrtStatus,
    pub mismatch_percentage: f64,
    pub threshold: f64,
    pub baseline_path: Option<String>,
    pub current_path: Option<String>,
    pub diff_path: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VrtStatus {
    Passed,
    Failed,
    NewBaseline,
    Error,
}

/// Full suite result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrtSuiteResult {
    pub suite_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub new_baselines: usize,
    pub errors: usize,
    pub results: Vec<VrtComparisonResult>,
    pub duration_ms: u64,
}

/// Compare two PNG images at the byte level.
/// Returns the mismatch percentage (0.0 = identical, 100.0 = completely different).
pub fn compare_images(baseline: &[u8], current: &[u8]) -> f64 {
    if baseline == current {
        return 0.0;
    }
    if baseline.is_empty() || current.is_empty() {
        return 100.0;
    }

    let min_len = baseline.len().min(current.len());
    let max_len = baseline.len().max(current.len());
    let mut different_bytes = 0u64;

    for i in 0..min_len {
        if baseline[i] != current[i] {
            different_bytes += 1;
        }
    }
    // Extra bytes count as different
    different_bytes += (max_len - min_len) as u64;

    (different_bytes as f64 / max_len as f64) * 100.0
}

/// Load a VRT suite from a JSON file.
pub fn load_suite(path: &str) -> Result<VrtSuite> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Cdp(format!("failed to read VRT suite: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| Error::Cdp(format!("failed to parse VRT suite: {e}")))
}

/// Save baseline image.
pub fn save_baseline(dir: &str, test_name: &str, data: &[u8]) -> Result<PathBuf> {
    let dir_path = Path::new(dir);
    std::fs::create_dir_all(dir_path)
        .map_err(|e| Error::Cdp(format!("failed to create baseline dir: {e}")))?;
    let file_name = format!("{}.png", sanitize_name(test_name));
    let path = dir_path.join(&file_name);
    std::fs::write(&path, data)
        .map_err(|e| Error::Cdp(format!("failed to write baseline: {e}")))?;
    Ok(path)
}

/// Load baseline image.
pub fn load_baseline(dir: &str, test_name: &str) -> Option<Vec<u8>> {
    let file_name = format!("{}.png", sanitize_name(test_name));
    let path = Path::new(dir).join(&file_name);
    std::fs::read(&path).ok()
}

/// Save current screenshot.
pub fn save_current(dir: &str, test_name: &str, data: &[u8]) -> Result<PathBuf> {
    let dir_path = Path::new(dir);
    std::fs::create_dir_all(dir_path)
        .map_err(|e| Error::Cdp(format!("failed to create output dir: {e}")))?;
    let file_name = format!("{}.png", sanitize_name(test_name));
    let path = dir_path.join(&file_name);
    std::fs::write(&path, data)
        .map_err(|e| Error::Cdp(format!("failed to write current: {e}")))?;
    Ok(path)
}

/// Save diff image.
pub fn save_diff(dir: &str, test_name: &str, data: &[u8]) -> Result<PathBuf> {
    let dir_path = Path::new(dir);
    std::fs::create_dir_all(dir_path)
        .map_err(|e| Error::Cdp(format!("failed to create diff dir: {e}")))?;
    let file_name = format!("{}-diff.png", sanitize_name(test_name));
    let path = dir_path.join(&file_name);
    std::fs::write(&path, data)
        .map_err(|e| Error::Cdp(format!("failed to write diff: {e}")))?;
    Ok(path)
}

/// Compare a single test case against its baseline.
pub fn compare_test(
    test: &VrtTestCase,
    current_data: &[u8],
    baseline_dir: &str,
    output_dir: &str,
    diff_dir: &str,
    suite_threshold: f64,
) -> VrtComparisonResult {
    let threshold = if test.threshold != default_threshold() {
        test.threshold
    } else {
        suite_threshold
    };

    // Save current
    let current_path = match save_current(output_dir, &test.name, current_data) {
        Ok(p) => Some(p.to_string_lossy().to_string()),
        Err(e) => {
            return VrtComparisonResult {
                test_name: test.name.clone(),
                status: VrtStatus::Error,
                mismatch_percentage: 0.0,
                threshold,
                baseline_path: None,
                current_path: None,
                diff_path: None,
                dimensions: None,
                error: Some(e.to_string()),
            }
        }
    };

    // Load baseline
    match load_baseline(baseline_dir, &test.name) {
        Some(baseline_data) => {
            let mismatch = compare_images(&baseline_data, current_data);
            let baseline_path =
                Some(format!("{}/{}.png", baseline_dir, sanitize_name(&test.name)));

            let diff_path = if mismatch > threshold {
                let diff_data = generate_diff_image(&baseline_data, current_data);
                save_diff(diff_dir, &test.name, &diff_data)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            } else {
                None
            };

            VrtComparisonResult {
                test_name: test.name.clone(),
                status: if mismatch <= threshold {
                    VrtStatus::Passed
                } else {
                    VrtStatus::Failed
                },
                mismatch_percentage: mismatch,
                threshold,
                baseline_path,
                current_path,
                diff_path,
                dimensions: None,
                error: None,
            }
        }
        None => {
            // No baseline exists — save current as new baseline
            let _ = save_baseline(baseline_dir, &test.name, current_data);
            VrtComparisonResult {
                test_name: test.name.clone(),
                status: VrtStatus::NewBaseline,
                mismatch_percentage: 0.0,
                threshold,
                baseline_path: Some(format!(
                    "{}/{}.png",
                    baseline_dir,
                    sanitize_name(&test.name)
                )),
                current_path,
                diff_path: None,
                dimensions: None,
                error: None,
            }
        }
    }
}

/// Generate a simple diff image (XOR-based).
fn generate_diff_image(baseline: &[u8], current: &[u8]) -> Vec<u8> {
    let min_len = baseline.len().min(current.len());
    let mut diff = vec![0u8; baseline.len().max(current.len())];
    for i in 0..min_len {
        diff[i] = baseline[i] ^ current[i];
    }
    // Fill extra with 0xFF for visibility
    for i in min_len..diff.len() {
        diff[i] = 0xFF;
    }
    diff
}

/// Validate a VRT suite definition.
pub fn validate_suite(suite: &VrtSuite) -> Vec<String> {
    let mut errors = Vec::new();
    if suite.name.is_empty() {
        errors.push("suite name is required".into());
    }
    if suite.tests.is_empty() {
        errors.push("suite must have at least one test".into());
    }
    let mut names = std::collections::HashSet::new();
    for (i, test) in suite.tests.iter().enumerate() {
        if test.name.is_empty() {
            errors.push(format!("test[{}]: name is required", i));
        }
        if test.url.is_empty() {
            errors.push(format!("test[{}]: url is required", i));
        }
        if test.threshold < 0.0 || test.threshold > 100.0 {
            errors.push(format!("test[{}]: threshold must be 0-100", i));
        }
        if !names.insert(&test.name) {
            errors.push(format!("test[{}]: duplicate name '{}'", i, test.name));
        }
    }
    errors
}

/// Generate a CI-compatible report (JUnit XML format).
pub fn generate_junit_report(result: &VrtSuiteResult) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" time=\"{:.3}\">\n",
        result.suite_name,
        result.total,
        result.failed,
        result.errors,
        result.duration_ms as f64 / 1000.0
    ));

    for r in &result.results {
        xml.push_str(&format!("  <testcase name=\"{}\"", r.test_name));
        match r.status {
            VrtStatus::Passed | VrtStatus::NewBaseline => {
                xml.push_str(" />\n");
            }
            VrtStatus::Failed => {
                xml.push_str(">\n");
                xml.push_str(&format!(
                    "    <failure message=\"Visual mismatch: {:.2}% (threshold: {:.2}%)\">",
                    r.mismatch_percentage, r.threshold
                ));
                if let Some(ref diff) = r.diff_path {
                    xml.push_str(&format!("\nDiff image: {}", diff));
                }
                xml.push_str("</failure>\n");
                xml.push_str("  </testcase>\n");
            }
            VrtStatus::Error => {
                xml.push_str(">\n");
                xml.push_str(&format!(
                    "    <error message=\"{}\"/>",
                    r.error.as_deref().unwrap_or("unknown error")
                ));
                xml.push_str("\n  </testcase>\n");
            }
        }
    }

    xml.push_str("</testsuite>\n");
    xml
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_identical_images() {
        let data = vec![1, 2, 3, 4, 5];
        assert_eq!(compare_images(&data, &data), 0.0);
    }

    #[test]
    fn compare_completely_different() {
        let a = vec![0u8; 100];
        let b = vec![255u8; 100];
        assert!(compare_images(&a, &b) > 99.0);
    }

    #[test]
    fn compare_different_lengths() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3, 4, 5];
        let mismatch = compare_images(&a, &b);
        assert!(mismatch > 0.0);
        assert!(mismatch <= 100.0);
    }

    #[test]
    fn compare_empty() {
        assert_eq!(compare_images(&[], &[1, 2, 3]), 100.0);
        assert_eq!(compare_images(&[1], &[]), 100.0);
    }

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_name("Hello World!"), "Hello_World_");
        assert_eq!(sanitize_name("test-case_1"), "test-case_1");
    }

    #[test]
    fn parse_suite() {
        let json = r#"{
            "name": "Homepage Tests",
            "baseline_dir": ".vrt/baselines",
            "tests": [
                { "name": "hero", "url": "https://example.com", "selector": ".hero", "threshold": 0.5 },
                { "name": "full-page", "url": "https://example.com", "full_page": true }
            ]
        }"#;
        let suite: VrtSuite = serde_json::from_str(json).unwrap();
        assert_eq!(suite.name, "Homepage Tests");
        assert_eq!(suite.tests.len(), 2);
        assert_eq!(suite.tests[0].threshold, 0.5);
        assert!(suite.tests[1].full_page);
    }

    #[test]
    fn validate_suite_ok() {
        let suite = VrtSuite {
            name: "test".into(),
            description: String::new(),
            baseline_dir: ".vrt".into(),
            output_dir: ".vrt/out".into(),
            diff_dir: ".vrt/diff".into(),
            threshold: 0.1,
            tests: vec![VrtTestCase {
                name: "home".into(),
                url: "https://example.com".into(),
                selector: None,
                full_page: false,
                threshold: 0.1,
                viewport: None,
                wait_for: None,
                hide_selectors: vec![],
                delay_ms: 0,
            }],
        };
        assert!(validate_suite(&suite).is_empty());
    }

    #[test]
    fn validate_suite_errors() {
        let suite = VrtSuite {
            name: String::new(),
            description: String::new(),
            baseline_dir: String::new(),
            output_dir: String::new(),
            diff_dir: String::new(),
            threshold: 0.1,
            tests: vec![],
        };
        let errors = validate_suite(&suite);
        assert!(errors.len() >= 2);
    }

    #[test]
    fn validate_duplicate_names() {
        let suite = VrtSuite {
            name: "test".into(),
            description: String::new(),
            baseline_dir: ".vrt".into(),
            output_dir: ".vrt/out".into(),
            diff_dir: ".vrt/diff".into(),
            threshold: 0.1,
            tests: vec![
                VrtTestCase {
                    name: "home".into(),
                    url: "https://a.com".into(),
                    selector: None,
                    full_page: false,
                    threshold: 0.1,
                    viewport: None,
                    wait_for: None,
                    hide_selectors: vec![],
                    delay_ms: 0,
                },
                VrtTestCase {
                    name: "home".into(),
                    url: "https://b.com".into(),
                    selector: None,
                    full_page: false,
                    threshold: 0.1,
                    viewport: None,
                    wait_for: None,
                    hide_selectors: vec![],
                    delay_ms: 0,
                },
            ],
        };
        let errors = validate_suite(&suite);
        assert!(errors.iter().any(|e| e.contains("duplicate")));
    }

    #[test]
    fn baseline_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let data = vec![137, 80, 78, 71, 1, 2, 3]; // Fake PNG
        let path = save_baseline(dir.path().to_str().unwrap(), "test-case", &data).unwrap();
        assert!(path.exists());
        let loaded = load_baseline(dir.path().to_str().unwrap(), "test-case").unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn compare_test_new_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_dir = dir.path().join("baselines");
        let output_dir = dir.path().join("current");
        let diff_dir = dir.path().join("diffs");
        let test = VrtTestCase {
            name: "new-test".into(),
            url: "https://example.com".into(),
            selector: None,
            full_page: false,
            threshold: 0.1,
            viewport: None,
            wait_for: None,
            hide_selectors: vec![],
            delay_ms: 0,
        };
        let data = vec![1, 2, 3, 4, 5];
        let result = compare_test(
            &test,
            &data,
            baseline_dir.to_str().unwrap(),
            output_dir.to_str().unwrap(),
            diff_dir.to_str().unwrap(),
            0.1,
        );
        assert_eq!(result.status, VrtStatus::NewBaseline);
    }

    #[test]
    fn compare_test_pass() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_dir = dir.path().join("baselines");
        let output_dir = dir.path().join("current");
        let diff_dir = dir.path().join("diffs");
        let test = VrtTestCase {
            name: "stable-test".into(),
            url: "https://example.com".into(),
            selector: None,
            full_page: false,
            threshold: 5.0,
            viewport: None,
            wait_for: None,
            hide_selectors: vec![],
            delay_ms: 0,
        };
        let data = vec![1, 2, 3, 4, 5];
        save_baseline(baseline_dir.to_str().unwrap(), "stable-test", &data).unwrap();
        let result = compare_test(
            &test,
            &data,
            baseline_dir.to_str().unwrap(),
            output_dir.to_str().unwrap(),
            diff_dir.to_str().unwrap(),
            5.0,
        );
        assert_eq!(result.status, VrtStatus::Passed);
        assert_eq!(result.mismatch_percentage, 0.0);
    }

    #[test]
    fn compare_test_fail() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_dir = dir.path().join("baselines");
        let output_dir = dir.path().join("current");
        let diff_dir = dir.path().join("diffs");
        let test = VrtTestCase {
            name: "changed-test".into(),
            url: "https://example.com".into(),
            selector: None,
            full_page: false,
            threshold: 0.1,
            viewport: None,
            wait_for: None,
            hide_selectors: vec![],
            delay_ms: 0,
        };
        let baseline = vec![0u8; 100];
        let current = vec![255u8; 100];
        save_baseline(baseline_dir.to_str().unwrap(), "changed-test", &baseline).unwrap();
        let result = compare_test(
            &test,
            &current,
            baseline_dir.to_str().unwrap(),
            output_dir.to_str().unwrap(),
            diff_dir.to_str().unwrap(),
            0.1,
        );
        assert_eq!(result.status, VrtStatus::Failed);
        assert!(result.mismatch_percentage > 0.1);
        assert!(result.diff_path.is_some());
    }

    #[test]
    fn junit_report() {
        let result = VrtSuiteResult {
            suite_name: "test".into(),
            total: 2,
            passed: 1,
            failed: 1,
            new_baselines: 0,
            errors: 0,
            results: vec![
                VrtComparisonResult {
                    test_name: "pass".into(),
                    status: VrtStatus::Passed,
                    mismatch_percentage: 0.0,
                    threshold: 0.1,
                    baseline_path: None,
                    current_path: None,
                    diff_path: None,
                    dimensions: None,
                    error: None,
                },
                VrtComparisonResult {
                    test_name: "fail".into(),
                    status: VrtStatus::Failed,
                    mismatch_percentage: 5.0,
                    threshold: 0.1,
                    baseline_path: None,
                    current_path: None,
                    diff_path: Some("/diffs/fail.png".into()),
                    dimensions: None,
                    error: None,
                },
            ],
            duration_ms: 1500,
        };
        let xml = generate_junit_report(&result);
        assert!(xml.contains("<testsuite"));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("Visual mismatch"));
    }
}
