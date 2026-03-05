use super::types::*;
use super::steps::*;
use std::collections::HashMap;

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
