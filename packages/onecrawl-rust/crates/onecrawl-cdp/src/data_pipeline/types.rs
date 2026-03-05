use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
