//! Data processing pipeline for transforming, filtering, and deduplicating
//! scraped data. Operates on `Vec<HashMap<String, String>>` items.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ─────────────────────────── Types ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStep {
    Filter {
        field: String,
        operator: String,
        value: String,
    },
    Transform {
        field: String,
        transform: String,
    },
    Rename {
        from: String,
        to: String,
    },
    Remove {
        fields: Vec<String>,
    },
    Add {
        field: String,
        value: String,
    },
    Deduplicate {
        field: String,
    },
    Sort {
        field: String,
        descending: bool,
    },
    Limit {
        count: usize,
    },
    Skip {
        count: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub input_count: usize,
    pub output_count: usize,
    pub filtered_count: usize,
    pub deduplicated_count: usize,
    pub errors: Vec<String>,
    pub items: Vec<HashMap<String, String>>,
}

// ─────────────────────────── Core ────────────────────────────

/// Execute all pipeline steps in order.
pub fn execute_pipeline(
    pipeline: &Pipeline,
    items: Vec<HashMap<String, String>>,
) -> PipelineResult {
    let input_count = items.len();
    let mut current = items;
    let mut total_filtered: usize = 0;
    let mut total_deduped: usize = 0;
    let mut errors: Vec<String> = Vec::new();

    for (i, step) in pipeline.steps.iter().enumerate() {
        let before = current.len();
        let (next, filtered) = apply_step(step, current);
        current = next;

        match step {
            PipelineStep::Deduplicate { .. } => {
                total_deduped += filtered;
            }
            PipelineStep::Filter { .. } => {
                total_filtered += filtered;
            }
            _ => {
                if filtered > 0 {
                    total_filtered += filtered;
                }
            }
        }

        let after = current.len();
        if after == 0 && before > 0 {
            errors.push(format!(
                "step {i} ({}) reduced items from {before} to 0",
                step_name(step)
            ));
        }
    }

    PipelineResult {
        input_count,
        output_count: current.len(),
        filtered_count: total_filtered,
        deduplicated_count: total_deduped,
        errors,
        items: current,
    }
}

/// Apply a single pipeline step. Returns (result_items, removed_count).
pub fn apply_step(
    step: &PipelineStep,
    items: Vec<HashMap<String, String>>,
) -> (Vec<HashMap<String, String>>, usize) {
    match step {
        PipelineStep::Filter {
            field,
            operator,
            value,
        } => {
            let before = items.len();
            let filtered: Vec<_> = items
                .into_iter()
                .filter(|item| {
                    match_filter(
                        item.get(field).map(|s| s.as_str()).unwrap_or(""),
                        operator,
                        value,
                    )
                })
                .collect();
            let removed = before - filtered.len();
            (filtered, removed)
        }

        PipelineStep::Transform { field, transform } => {
            let out: Vec<_> = items
                .into_iter()
                .map(|mut item| {
                    if let Some(val) = item.get(field).cloned() {
                        item.insert(field.clone(), apply_transform(&val, transform));
                    }
                    item
                })
                .collect();
            (out, 0)
        }

        PipelineStep::Rename { from, to } => {
            let out: Vec<_> = items
                .into_iter()
                .map(|mut item| {
                    if let Some(val) = item.remove(from) {
                        item.insert(to.clone(), val);
                    }
                    item
                })
                .collect();
            (out, 0)
        }

        PipelineStep::Remove { fields } => {
            let out: Vec<_> = items
                .into_iter()
                .map(|mut item| {
                    for f in fields {
                        item.remove(f);
                    }
                    item
                })
                .collect();
            (out, 0)
        }

        PipelineStep::Add { field, value } => {
            let out: Vec<_> = items
                .into_iter()
                .map(|mut item| {
                    item.insert(field.clone(), value.clone());
                    item
                })
                .collect();
            (out, 0)
        }

        PipelineStep::Deduplicate { field } => {
            let before = items.len();
            let mut seen = HashSet::new();
            let deduped: Vec<_> = items
                .into_iter()
                .filter(|item| {
                    let key = item.get(field).cloned().unwrap_or_default();
                    seen.insert(key)
                })
                .collect();
            let removed = before - deduped.len();
            (deduped, removed)
        }

        PipelineStep::Sort { field, descending } => {
            let mut sorted = items;
            let desc = *descending;
            let f = field.clone();
            sorted.sort_by(|a, b| {
                let va = a.get(&f).cloned().unwrap_or_default();
                let vb = b.get(&f).cloned().unwrap_or_default();
                // Try numeric comparison first
                let cmp = match (va.parse::<f64>(), vb.parse::<f64>()) {
                    (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
                    _ => va.cmp(&vb),
                };
                if desc { cmp.reverse() } else { cmp }
            });
            (sorted, 0)
        }

        PipelineStep::Limit { count } => {
            let before = items.len();
            let limited: Vec<_> = items.into_iter().take(*count).collect();
            let removed = before - limited.len();
            (limited, removed)
        }

        PipelineStep::Skip { count } => {
            let before = items.len();
            let skipped: Vec<_> = items.into_iter().skip(*count).collect();
            let removed = before - skipped.len();
            (skipped, removed)
        }
    }
}

// ─────────────────────────── Validation ──────────────────────

/// Validate a pipeline configuration, returning a list of error strings.
pub fn validate_pipeline(pipeline: &Pipeline) -> Vec<String> {
    let mut errors = Vec::new();

    if pipeline.name.is_empty() {
        errors.push("pipeline name is empty".into());
    }
    if pipeline.steps.is_empty() {
        errors.push("pipeline has no steps".into());
    }

    for (i, step) in pipeline.steps.iter().enumerate() {
        match step {
            PipelineStep::Filter {
                field, operator, ..
            } => {
                if field.is_empty() {
                    errors.push(format!("step {i}: filter field is empty"));
                }
                let valid = [
                    "eq",
                    "neq",
                    "contains",
                    "starts_with",
                    "ends_with",
                    "regex",
                    "gt",
                    "lt",
                    "empty",
                    "not_empty",
                ];
                if !valid.contains(&operator.as_str()) {
                    errors.push(format!(
                        "step {i}: unknown filter operator '{operator}'. Valid: {valid:?}"
                    ));
                }
            }
            PipelineStep::Transform { field, transform } => {
                if field.is_empty() {
                    errors.push(format!("step {i}: transform field is empty"));
                }
                let base = transform.split(':').next().unwrap_or("");
                let valid = [
                    "trim",
                    "lowercase",
                    "uppercase",
                    "strip_html",
                    "strip_whitespace",
                    "truncate",
                    "replace",
                    "split",
                ];
                if !valid.contains(&base) {
                    errors.push(format!(
                        "step {i}: unknown transform '{base}'. Valid: {valid:?}"
                    ));
                }
            }
            PipelineStep::Rename { from, to } => {
                if from.is_empty() {
                    errors.push(format!("step {i}: rename 'from' is empty"));
                }
                if to.is_empty() {
                    errors.push(format!("step {i}: rename 'to' is empty"));
                }
            }
            PipelineStep::Remove { fields } => {
                if fields.is_empty() {
                    errors.push(format!("step {i}: remove fields list is empty"));
                }
            }
            PipelineStep::Add { field, .. } => {
                if field.is_empty() {
                    errors.push(format!("step {i}: add field name is empty"));
                }
            }
            PipelineStep::Deduplicate { field } => {
                if field.is_empty() {
                    errors.push(format!("step {i}: deduplicate field is empty"));
                }
            }
            PipelineStep::Sort { field, .. } => {
                if field.is_empty() {
                    errors.push(format!("step {i}: sort field is empty"));
                }
            }
            PipelineStep::Limit { count } => {
                if *count == 0 {
                    errors.push(format!("step {i}: limit count is 0"));
                }
            }
            PipelineStep::Skip { .. } => {}
        }
    }

    errors
}

// ─────────────────────────── Persistence ─────────────────────

/// Save a pipeline definition to a JSON file.
pub fn save_pipeline(pipeline: &Pipeline, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(pipeline)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a pipeline definition from a JSON file.
pub fn load_pipeline(path: &Path) -> Result<Pipeline> {
    let data = std::fs::read_to_string(path)?;
    let pipeline: Pipeline = serde_json::from_str(&data)?;
    Ok(pipeline)
}

// ─────────────────────────── Export ──────────────────────────

/// Export processed results to a file. Supported formats: "json", "jsonl", "csv".
/// Returns the number of items written.
pub fn export_processed(result: &PipelineResult, path: &Path, format: &str) -> Result<usize> {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&result.items)?;
            std::fs::write(path, json)?;
            Ok(result.items.len())
        }
        "jsonl" => {
            let mut out = String::new();
            for item in &result.items {
                out.push_str(&serde_json::to_string(item)?);
                out.push('\n');
            }
            std::fs::write(path, out)?;
            Ok(result.items.len())
        }
        "csv" => export_csv(&result.items, path),
        _ => Err(Error::InvalidInput(format!(
            "unsupported export format '{format}'. Use: json, jsonl, csv"
        ))),
    }
}

// ─────────────────────────── Helpers ─────────────────────────

fn step_name(step: &PipelineStep) -> &'static str {
    match step {
        PipelineStep::Filter { .. } => "filter",
        PipelineStep::Transform { .. } => "transform",
        PipelineStep::Rename { .. } => "rename",
        PipelineStep::Remove { .. } => "remove",
        PipelineStep::Add { .. } => "add",
        PipelineStep::Deduplicate { .. } => "deduplicate",
        PipelineStep::Sort { .. } => "sort",
        PipelineStep::Limit { .. } => "limit",
        PipelineStep::Skip { .. } => "skip",
    }
}

fn match_filter(val: &str, operator: &str, target: &str) -> bool {
    match operator {
        "eq" => val == target,
        "neq" => val != target,
        "contains" => val.contains(target),
        "starts_with" => val.starts_with(target),
        "ends_with" => val.ends_with(target),
        "regex" => simple_regex_match(val, target),
        "gt" => match (val.parse::<f64>(), target.parse::<f64>()) {
            (Ok(a), Ok(b)) => a > b,
            _ => val > target,
        },
        "lt" => match (val.parse::<f64>(), target.parse::<f64>()) {
            (Ok(a), Ok(b)) => a < b,
            _ => val < target,
        },
        "empty" => val.is_empty(),
        "not_empty" => !val.is_empty(),
        _ => false,
    }
}

/// Basic regex-like matching without the regex crate.
/// Supports: `^` (start), `$` (end), `.*` (any), literal chars.
fn simple_regex_match(text: &str, pattern: &str) -> bool {
    let anchored_start = pattern.starts_with('^');
    let anchored_end = pattern.ends_with('$');
    let inner = pattern
        .strip_prefix('^')
        .unwrap_or(pattern)
        .strip_suffix('$')
        .unwrap_or(pattern.strip_prefix('^').unwrap_or(pattern));

    if inner == ".*" {
        return true;
    }

    let parts: Vec<&str> = inner.split(".*").collect();

    if parts.len() == 1 {
        // No wildcard — literal match with anchors
        if anchored_start && anchored_end {
            return text == inner;
        } else if anchored_start {
            return text.starts_with(inner);
        } else if anchored_end {
            return text.ends_with(inner);
        }
        return text.contains(inner);
    }

    // Multiple segments separated by .*
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && anchored_start {
            if !text[pos..].starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if let Some(found) = text[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }

    if anchored_end
        && let Some(last) = parts.last()
        && !last.is_empty()
    {
        return text.ends_with(last);
    }

    true
}

fn apply_transform(val: &str, transform: &str) -> String {
    let parts: Vec<&str> = transform.splitn(3, ':').collect();
    let op = parts[0];

    match op {
        "trim" => val.trim().to_string(),
        "lowercase" => val.to_lowercase(),
        "uppercase" => val.to_uppercase(),
        "strip_html" => strip_html_tags(val),
        "strip_whitespace" => val.split_whitespace().collect::<Vec<_>>().join(" "),
        "truncate" => {
            let n: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
            if val.len() <= n {
                val.to_string()
            } else {
                val.chars().take(n).collect()
            }
        }
        "replace" => {
            let old = parts.get(1).unwrap_or(&"");
            let new = parts.get(2).unwrap_or(&"");
            val.replace(old, new)
        }
        "split" => {
            let delim = parts.get(1).unwrap_or(&",");
            let index: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            val.split(delim).nth(index).unwrap_or("").to_string()
        }
        _ => val.to_string(),
    }
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut inside_tag = false;
    for ch in s.chars() {
        if ch == '<' {
            inside_tag = true;
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            out.push(ch);
        }
    }
    out
}

fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn export_csv(items: &[HashMap<String, String>], path: &Path) -> Result<usize> {
    if items.is_empty() {
        std::fs::write(path, "")?;
        return Ok(0);
    }

    let mut columns: Vec<String> = Vec::new();
    for item in items {
        for key in item.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }
    columns.sort();

    let mut csv = String::new();
    csv.push_str(
        &columns
            .iter()
            .map(|c| escape_csv_field(c))
            .collect::<Vec<_>>()
            .join(","),
    );
    csv.push('\n');

    for item in items {
        let row: Vec<String> = columns
            .iter()
            .map(|col| escape_csv_field(item.get(col).map(|s| s.as_str()).unwrap_or("")))
            .collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    std::fs::write(path, &csv)?;
    Ok(items.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn sample_items() -> Vec<HashMap<String, String>> {
        vec![
            item(&[("name", "Alice"), ("age", "30"), ("city", "Rome")]),
            item(&[("name", "Bob"), ("age", "25"), ("city", "Milan")]),
            item(&[("name", "Charlie"), ("age", "35"), ("city", "Rome")]),
        ]
    }

    // ── Filter ──

    #[test]
    fn test_filter_eq() {
        let step = PipelineStep::Filter {
            field: "city".into(),
            operator: "eq".into(),
            value: "Rome".into(),
        };
        let (out, removed) = apply_step(&step, sample_items());
        assert_eq!(out.len(), 2);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_filter_contains() {
        let step = PipelineStep::Filter {
            field: "name".into(),
            operator: "contains".into(),
            value: "li".into(),
        };
        let (out, _) = apply_step(&step, sample_items());
        assert_eq!(out.len(), 2); // Alice, Charlie
    }

    #[test]
    fn test_filter_empty() {
        let items = vec![
            item(&[("name", "Alice"), ("bio", "")]),
            item(&[("name", "Bob"), ("bio", "hello")]),
        ];
        let step = PipelineStep::Filter {
            field: "bio".into(),
            operator: "empty".into(),
            value: String::new(),
        };
        let (out, _) = apply_step(&step, items);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["name"], "Alice");
    }

    #[test]
    fn test_filter_not_empty() {
        let items = vec![
            item(&[("name", "Alice"), ("bio", "")]),
            item(&[("name", "Bob"), ("bio", "hello")]),
        ];
        let step = PipelineStep::Filter {
            field: "bio".into(),
            operator: "not_empty".into(),
            value: String::new(),
        };
        let (out, _) = apply_step(&step, items);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["name"], "Bob");
    }

    // ── Transform ──

    #[test]
    fn test_transform_trim() {
        let items = vec![item(&[("name", "  Alice  ")])];
        let step = PipelineStep::Transform {
            field: "name".into(),
            transform: "trim".into(),
        };
        let (out, _) = apply_step(&step, items);
        assert_eq!(out[0]["name"], "Alice");
    }

    #[test]
    fn test_transform_lowercase() {
        let items = vec![item(&[("name", "ALICE")])];
        let step = PipelineStep::Transform {
            field: "name".into(),
            transform: "lowercase".into(),
        };
        let (out, _) = apply_step(&step, items);
        assert_eq!(out[0]["name"], "alice");
    }

    #[test]
    fn test_transform_uppercase() {
        let items = vec![item(&[("name", "alice")])];
        let step = PipelineStep::Transform {
            field: "name".into(),
            transform: "uppercase".into(),
        };
        let (out, _) = apply_step(&step, items);
        assert_eq!(out[0]["name"], "ALICE");
    }

    // ── Rename ──

    #[test]
    fn test_rename() {
        let items = vec![item(&[("name", "Alice")])];
        let step = PipelineStep::Rename {
            from: "name".into(),
            to: "full_name".into(),
        };
        let (out, _) = apply_step(&step, items);
        assert!(out[0].contains_key("full_name"));
        assert!(!out[0].contains_key("name"));
        assert_eq!(out[0]["full_name"], "Alice");
    }

    // ── Remove ──

    #[test]
    fn test_remove() {
        let items = vec![item(&[("name", "Alice"), ("age", "30"), ("city", "Rome")])];
        let step = PipelineStep::Remove {
            fields: vec!["age".into(), "city".into()],
        };
        let (out, _) = apply_step(&step, items);
        assert_eq!(out[0].len(), 1);
        assert!(out[0].contains_key("name"));
    }

    // ── Deduplicate ──

    #[test]
    fn test_deduplicate() {
        let items = vec![
            item(&[("city", "Rome")]),
            item(&[("city", "Milan")]),
            item(&[("city", "Rome")]),
        ];
        let step = PipelineStep::Deduplicate {
            field: "city".into(),
        };
        let (out, removed) = apply_step(&step, items);
        assert_eq!(out.len(), 2);
        assert_eq!(removed, 1);
    }

    // ── Sort ──

    #[test]
    fn test_sort_ascending() {
        let step = PipelineStep::Sort {
            field: "age".into(),
            descending: false,
        };
        let (out, _) = apply_step(&step, sample_items());
        assert_eq!(out[0]["age"], "25");
        assert_eq!(out[1]["age"], "30");
        assert_eq!(out[2]["age"], "35");
    }

    #[test]
    fn test_sort_descending() {
        let step = PipelineStep::Sort {
            field: "age".into(),
            descending: true,
        };
        let (out, _) = apply_step(&step, sample_items());
        assert_eq!(out[0]["age"], "35");
        assert_eq!(out[2]["age"], "25");
    }

    // ── Limit & Skip ──

    #[test]
    fn test_limit() {
        let step = PipelineStep::Limit { count: 2 };
        let (out, removed) = apply_step(&step, sample_items());
        assert_eq!(out.len(), 2);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_skip() {
        let step = PipelineStep::Skip { count: 1 };
        let (out, removed) = apply_step(&step, sample_items());
        assert_eq!(out.len(), 2);
        assert_eq!(removed, 1);
    }

    // ── Full pipeline ──

    #[test]
    fn test_full_pipeline_execution() {
        let pipeline = Pipeline {
            name: "test".into(),
            steps: vec![
                PipelineStep::Filter {
                    field: "city".into(),
                    operator: "eq".into(),
                    value: "Rome".into(),
                },
                PipelineStep::Transform {
                    field: "name".into(),
                    transform: "uppercase".into(),
                },
                PipelineStep::Sort {
                    field: "age".into(),
                    descending: false,
                },
            ],
        };
        let result = execute_pipeline(&pipeline, sample_items());
        assert_eq!(result.input_count, 3);
        assert_eq!(result.output_count, 2);
        assert_eq!(result.filtered_count, 1);
        assert_eq!(result.items[0]["name"], "ALICE");
        assert_eq!(result.items[1]["name"], "CHARLIE");
    }

    // ── CSV export ──

    #[test]
    fn test_csv_export() {
        let items = vec![
            item(&[("name", "Alice"), ("age", "30")]),
            item(&[("name", "Bob"), ("age", "25")]),
        ];
        let tmp = std::env::temp_dir().join("onecrawl_pipeline_csv_test.csv");
        let count = export_csv(&items, &tmp).unwrap();
        assert_eq!(count, 2);
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.starts_with("age,name\n"));
        assert!(content.contains("30,Alice"));
        assert!(content.contains("25,Bob"));
        std::fs::remove_file(&tmp).ok();
    }

    // ── Validate ──

    #[test]
    fn test_validate_valid_pipeline() {
        let pipeline = Pipeline {
            name: "ok".into(),
            steps: vec![PipelineStep::Filter {
                field: "x".into(),
                operator: "eq".into(),
                value: "y".into(),
            }],
        };
        assert!(validate_pipeline(&pipeline).is_empty());
    }

    #[test]
    fn test_validate_empty_pipeline() {
        let pipeline = Pipeline {
            name: String::new(),
            steps: vec![],
        };
        let errors = validate_pipeline(&pipeline);
        assert!(errors.iter().any(|e| e.contains("name is empty")));
        assert!(errors.iter().any(|e| e.contains("no steps")));
    }

    #[test]
    fn test_validate_bad_operator() {
        let pipeline = Pipeline {
            name: "p".into(),
            steps: vec![PipelineStep::Filter {
                field: "x".into(),
                operator: "invalid_op".into(),
                value: "y".into(),
            }],
        };
        let errors = validate_pipeline(&pipeline);
        assert!(errors.iter().any(|e| e.contains("unknown filter operator")));
    }
}
