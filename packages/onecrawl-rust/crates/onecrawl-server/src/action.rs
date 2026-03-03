use serde::{Deserialize, Serialize};

/// Tagged action enum for the `/tabs/{id}/action` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    #[serde(rename = "click")]
    Click { ref_id: String },
    #[serde(rename = "type")]
    Type { ref_id: String, text: String },
    #[serde(rename = "fill")]
    Fill { ref_id: String, text: String },
    #[serde(rename = "press")]
    Press {
        key: String,
        ref_id: Option<String>,
    },
    #[serde(rename = "hover")]
    Hover { ref_id: String },
    #[serde(rename = "focus")]
    Focus { ref_id: String },
    #[serde(rename = "scroll")]
    Scroll {
        ref_id: Option<String>,
        pixels: Option<i64>,
    },
    #[serde(rename = "select")]
    Select { ref_id: String, value: String },
    #[serde(rename = "wait")]
    Wait { time: u64 },
    #[serde(rename = "actions")]
    Batch { actions: Vec<Action> },
}

/// Result from executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ActionResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(msg.into()),
        }
    }
}

/// Parse "e5" -> 5, "e123" -> 123, etc.
pub fn parse_ref_id(ref_id: &str) -> Result<i64, String> {
    ref_id
        .strip_prefix('e')
        .and_then(|n| n.parse::<i64>().ok())
        .ok_or_else(|| format!("invalid ref_id: {ref_id}"))
}
