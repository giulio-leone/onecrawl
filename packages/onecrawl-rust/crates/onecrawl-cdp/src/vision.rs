//! Streaming AI Vision — feeds browser screencast frames to a vision-language
//! model for continuous page understanding and reactive automation.
//!
//! The module is **model-agnostic**: it captures frames, prepares structured API
//! payloads ([`VisionRequest`]), and parses responses — but never calls an
//! external API itself. The caller (MCP client, CLI, or agent loop) is
//! responsible for the actual LLM round-trip.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ──────────────────────────── Configuration ────────────────────────────

/// Vision model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Model identifier (e.g. "gemini-2.5-pro", "gpt-4o", "claude-sonnet").
    #[serde(default = "default_model")]
    pub model: String,
    /// API key (or use env var).
    pub api_key: Option<String>,
    /// Custom API endpoint.
    pub api_url: Option<String>,
    /// Frames per second to capture (default 0.5 = every 2 s).
    #[serde(default = "default_fps")]
    pub fps: f32,
    /// Maximum FPS cap (default 2.0).
    #[serde(default = "default_max_fps")]
    pub max_fps: f32,
    /// Continuously describe what's on screen.
    #[serde(default)]
    pub describe: bool,
    /// What to react to: "errors", "popups", "captchas", "changes".
    #[serde(default)]
    pub react_to: Vec<String>,
    /// Custom system prompt for the model.
    pub prompt: Option<String>,
    /// Max tokens per response.
    pub max_tokens: Option<u32>,
    /// Cost cap in cents.
    pub max_cost_cents: Option<u32>,
    /// Screenshot format (JPEG by default for lower cost).
    #[serde(default)]
    pub screenshot_format: ScreenshotFormat,
    /// Downscale resolution for cost savings (width, height).
    pub resolution: Option<(u32, u32)>,
    /// Path to log descriptions.
    pub output_log: Option<String>,
}

fn default_model() -> String {
    "gpt-4o".to_string()
}
fn default_fps() -> f32 {
    0.5
}
fn default_max_fps() -> f32 {
    2.0
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            api_key: None,
            api_url: None,
            fps: default_fps(),
            max_fps: default_max_fps(),
            describe: false,
            react_to: Vec::new(),
            prompt: None,
            max_tokens: None,
            max_cost_cents: None,
            screenshot_format: ScreenshotFormat::default(),
            resolution: None,
            output_log: None,
        }
    }
}

/// Screenshot format for captured frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScreenshotFormat {
    Png,
    Jpeg { quality: u8 },
}

impl Default for ScreenshotFormat {
    fn default() -> Self {
        ScreenshotFormat::Jpeg { quality: 70 }
    }
}

// ──────────────────────────── Observation types ────────────────────────────

/// A vision observation — what the model saw.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionObservation {
    pub timestamp: String,
    pub frame_index: u64,
    pub description: String,
    pub detected: Vec<Detection>,
    pub suggested_actions: Vec<SuggestedAction>,
    pub confidence: f32,
    pub tokens_used: u32,
    pub cost_cents: u32,
}

/// A detected element or issue within a frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    /// Category: "error_dialog", "captcha", "popup", "loading", "content_change".
    pub category: String,
    pub description: String,
    pub confidence: f32,
    pub bounding_box: Option<BoundingBox>,
}

/// Pixel-space bounding box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// An action the model suggests the agent should take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    /// "click", "dismiss", "solve_captcha", "wait", "navigate".
    pub action_type: String,
    pub target: Option<String>,
    pub reasoning: String,
}

// ──────────────────────────── Status ────────────────────────────

/// Vision stream runtime status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionStatus {
    pub running: bool,
    pub frames_captured: u64,
    pub observations_made: u64,
    pub total_tokens: u64,
    pub total_cost_cents: u32,
    pub fps: f32,
    pub uptime_secs: f64,
    pub last_observation: Option<VisionObservation>,
    pub detections_by_category: HashMap<String, u64>,
}

// ──────────────────────────── API request/response ────────────────────────

/// Model-agnostic vision API request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRequest {
    pub model: String,
    pub messages: Vec<VisionMessage>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionMessage {
    pub role: String,
    pub content: Vec<VisionContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VisionContent {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// data:image/jpeg;base64,{data}
    pub url: String,
    /// "low", "high", "auto"
    pub detail: Option<String>,
}

// ──────────────────────────── VisionStream ────────────────────────────

/// Internal stats accumulator.
struct VisionStreamStats {
    frames_captured: u64,
    observations_made: u64,
    total_tokens: u64,
    total_cost_cents: u32,
    started_at: std::time::Instant,
    detections_by_category: HashMap<String, u64>,
}

impl VisionStreamStats {
    fn new() -> Self {
        Self {
            frames_captured: 0,
            observations_made: 0,
            total_tokens: 0,
            total_cost_cents: 0,
            started_at: std::time::Instant::now(),
            detections_by_category: HashMap::new(),
        }
    }
}

const MAX_OBSERVATIONS: usize = 1000;

/// Streaming vision pipeline.
///
/// Captures browser frames at a configurable FPS, prepares vision API payloads,
/// and stores parsed observations for downstream consumers.
pub struct VisionStream {
    config: Arc<RwLock<VisionConfig>>,
    running: Arc<RwLock<bool>>,
    observations: Arc<RwLock<Vec<VisionObservation>>>,
    stats: Arc<RwLock<VisionStreamStats>>,
}

impl VisionStream {
    pub fn new(config: VisionConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(RwLock::new(false)),
            observations: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(VisionStreamStats::new())),
        }
    }

    /// Start the vision stream.
    ///
    /// Captures frames from the CDP page at the configured FPS, prepares vision
    /// API request payloads, and stores placeholder observations. The caller
    /// should use [`prepare_vision_request`] + their own LLM client, then feed
    /// the response through [`parse_vision_response`].
    pub async fn start(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<(), String> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err("vision stream is already running".into());
            }
            *running = true;
        }

        // Reset stats
        {
            let mut stats = self.stats.write().await;
            *stats = VisionStreamStats::new();
        }

        let config = self.config.read().await.clone();
        let _initial_interval = std::time::Duration::from_secs_f64(1.0 / config.fps as f64);

        let page = page.clone();
        let running = Arc::clone(&self.running);
        let observations = Arc::clone(&self.observations);
        let stats = Arc::clone(&self.stats);
        let config_arc = Arc::clone(&self.config);

        tokio::spawn(async move {
            let mut frame_index: u64 = 0;
            loop {
                // Check if we should stop
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        break;
                    }
                }

                // Read current config (FPS may have changed)
                let cfg = config_arc.read().await.clone();
                let current_interval =
                    std::time::Duration::from_secs_f64(1.0 / cfg.fps as f64);

                // Build screencast options from config
                let opts = screencast_opts_from_config(&cfg);

                // Capture frame
                let frame_result =
                    crate::screencast::capture_frame(&page, &opts).await;
                let frame_bytes = match frame_result {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        tokio::time::sleep(current_interval).await;
                        continue;
                    }
                };

                frame_index += 1;

                // Update frame count
                {
                    let mut s = stats.write().await;
                    s.frames_captured += 1;
                }

                // Encode frame as base64
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&frame_bytes);

                // Build a placeholder observation (the actual LLM call happens externally)
                let mime = match cfg.screenshot_format {
                    ScreenshotFormat::Png => "image/png",
                    ScreenshotFormat::Jpeg { .. } => "image/jpeg",
                };
                let timestamp = chrono_now();

                let obs = VisionObservation {
                    timestamp,
                    frame_index,
                    description: format!(
                        "[frame #{frame_index}] captured ({} bytes, {mime})",
                        frame_bytes.len()
                    ),
                    detected: Vec::new(),
                    suggested_actions: Vec::new(),
                    confidence: 0.0,
                    tokens_used: 0,
                    cost_cents: 0,
                };

                // Store the observation (ring-buffer capped at MAX_OBSERVATIONS)
                {
                    let mut obs_vec = observations.write().await;
                    obs_vec.push(obs);
                    if obs_vec.len() > MAX_OBSERVATIONS {
                        let excess = obs_vec.len() - MAX_OBSERVATIONS;
                        obs_vec.drain(..excess);
                    }
                }

                // Update stats
                {
                    let mut s = stats.write().await;
                    s.observations_made += 1;
                }

                // Log to file if configured
                if let Some(ref log_path) = cfg.output_log {
                    let _ = append_log(
                        log_path,
                        &format!(
                            "frame={frame_index} size={} format={mime} base64_len={}\n",
                            frame_bytes.len(),
                            b64.len()
                        ),
                    );
                }

                // Check cost cap
                {
                    let s = stats.read().await;
                    if let Some(cap) = cfg.max_cost_cents {
                        if s.total_cost_cents >= cap {
                            drop(s);
                            let mut r = running.write().await;
                            *r = false;
                            break;
                        }
                    }
                }

                tokio::time::sleep(current_interval).await;
            }
        });

        Ok(())
    }

    /// Stop the vision stream.
    pub async fn stop(&self) -> Result<VisionStatus, String> {
        {
            let mut running = self.running.write().await;
            if !*running {
                return Err("vision stream is not running".into());
            }
            *running = false;
        }
        // Brief yield so the spawned task notices the flag
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(self.status().await)
    }

    /// Get current status.
    pub async fn status(&self) -> VisionStatus {
        let running = *self.running.read().await;
        let stats = self.stats.read().await;
        let obs = self.observations.read().await;
        let config = self.config.read().await;

        VisionStatus {
            running,
            frames_captured: stats.frames_captured,
            observations_made: stats.observations_made,
            total_tokens: stats.total_tokens,
            total_cost_cents: stats.total_cost_cents,
            fps: config.fps,
            uptime_secs: stats.started_at.elapsed().as_secs_f64(),
            last_observation: obs.last().cloned(),
            detections_by_category: stats.detections_by_category.clone(),
        }
    }

    /// Get recent observations (most recent first).
    pub async fn observations(&self, limit: usize) -> Vec<VisionObservation> {
        let obs = self.observations.read().await;
        let start = obs.len().saturating_sub(limit);
        obs[start..].iter().rev().cloned().collect()
    }

    /// Capture a single frame and produce a placeholder observation (one-shot).
    pub async fn describe_once(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<VisionObservation, String> {
        let config = self.config.read().await.clone();
        let opts = screencast_opts_from_config(&config);
        let frame_bytes = crate::screencast::capture_frame(page, &opts)
            .await
            .map_err(|e| format!("capture_frame failed: {e}"))?;

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&frame_bytes);

        let mime = match config.screenshot_format {
            ScreenshotFormat::Png => "image/png",
            ScreenshotFormat::Jpeg { .. } => "image/jpeg",
        };

        let request = self.prepare_vision_request(&b64, None);

        Ok(VisionObservation {
            timestamp: chrono_now(),
            frame_index: 0,
            description: format!(
                "[one-shot] captured ({} bytes, {mime}). Prepared request for model '{}' with {} message(s).",
                frame_bytes.len(),
                request.model,
                request.messages.len()
            ),
            detected: Vec::new(),
            suggested_actions: Vec::new(),
            confidence: 0.0,
            tokens_used: 0,
            cost_cents: 0,
        })
    }

    /// Prepare a vision API request payload (for external caller).
    pub fn prepare_vision_request(
        &self,
        frame_base64: &str,
        additional_prompt: Option<&str>,
    ) -> VisionRequest {
        // We need sync access — grab a blocking read since this is not async.
        // The caller should hold a config snapshot for hot paths.
        let config = self.config.blocking_read();
        build_vision_request(&config, frame_base64, additional_prompt)
    }

    /// Async variant of [`prepare_vision_request`].
    pub async fn prepare_vision_request_async(
        &self,
        frame_base64: &str,
        additional_prompt: Option<&str>,
    ) -> VisionRequest {
        let config = self.config.read().await;
        build_vision_request(&config, frame_base64, additional_prompt)
    }

    /// Parse a vision API response into a structured observation.
    pub fn parse_vision_response(
        &self,
        response_text: &str,
        frame_index: u64,
    ) -> Result<VisionObservation, String> {
        // Try to parse structured JSON first
        if let Ok(obs) = serde_json::from_str::<VisionObservation>(response_text) {
            return Ok(obs);
        }

        // Otherwise build an observation from freeform text
        let mut detected = Vec::new();
        let lower = response_text.to_lowercase();

        for (keyword, category) in &[
            ("error", "error_dialog"),
            ("captcha", "captcha"),
            ("popup", "popup"),
            ("modal", "popup"),
            ("loading", "loading"),
            ("spinner", "loading"),
            ("changed", "content_change"),
            ("cookie", "popup"),
            ("consent", "popup"),
        ] {
            if lower.contains(keyword) {
                detected.push(Detection {
                    category: category.to_string(),
                    description: format!("Detected keyword '{keyword}' in model response"),
                    confidence: 0.6,
                    bounding_box: None,
                });
            }
        }

        let suggested_actions = extract_suggested_actions(&lower);

        let confidence = if detected.is_empty() { 0.5 } else { 0.7 };
        Ok(VisionObservation {
            timestamp: chrono_now(),
            frame_index,
            description: response_text.to_string(),
            detected,
            suggested_actions,
            confidence,
            tokens_used: 0,
            cost_cents: 0,
        })
    }

    /// Record an externally-obtained observation (after calling the LLM).
    pub async fn record_observation(&self, obs: VisionObservation) {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.observations_made += 1;
            stats.total_tokens += obs.tokens_used as u64;
            stats.total_cost_cents += obs.cost_cents;
            for det in &obs.detected {
                *stats.detections_by_category.entry(det.category.clone()).or_insert(0) += 1;
            }
        }

        // Store observation
        {
            let mut obs_vec = self.observations.write().await;
            obs_vec.push(obs);
            if obs_vec.len() > MAX_OBSERVATIONS {
                let excess = obs_vec.len() - MAX_OBSERVATIONS;
                obs_vec.drain(..excess);
            }
        }
    }

    /// Update FPS at runtime.
    pub async fn set_fps(&self, fps: f32) -> Result<(), String> {
        let max_fps = self.config.read().await.max_fps;
        if fps <= 0.0 || fps > max_fps {
            return Err(format!("FPS must be between 0 and {max_fps}"));
        }
        self.config.write().await.fps = fps;
        Ok(())
    }

    /// Check if any recent detections match the configured `react_to` patterns.
    pub async fn matching_detections(&self, limit: usize) -> Vec<Detection> {
        let config = self.config.read().await;
        if config.react_to.is_empty() {
            return Vec::new();
        }
        let obs = self.observations.read().await;
        let start = obs.len().saturating_sub(limit);
        obs[start..]
            .iter()
            .flat_map(|o| &o.detected)
            .filter(|d| {
                config.react_to.iter().any(|r| {
                    d.category.contains(r) || r.contains(&d.category)
                })
            })
            .cloned()
            .collect()
    }
}

// ──────────────────────────── Helpers ────────────────────────────

fn screencast_opts_from_config(config: &VisionConfig) -> crate::screencast::ScreencastOptions {
    let (format, quality) = match config.screenshot_format {
        ScreenshotFormat::Png => ("png".to_string(), None),
        ScreenshotFormat::Jpeg { quality } => ("jpeg".to_string(), Some(quality as u32)),
    };
    let (max_width, max_height) = config
        .resolution
        .map(|(w, h)| (Some(w), Some(h)))
        .unwrap_or((Some(1280), Some(720)));
    crate::screencast::ScreencastOptions {
        format,
        quality,
        max_width,
        max_height,
        every_nth_frame: Some(1),
    }
}

fn build_vision_request(
    config: &VisionConfig,
    frame_base64: &str,
    additional_prompt: Option<&str>,
) -> VisionRequest {
    let mime = match config.screenshot_format {
        ScreenshotFormat::Png => "image/png",
        ScreenshotFormat::Jpeg { .. } => "image/jpeg",
    };

    let system_prompt = config.prompt.clone().unwrap_or_else(|| {
        let mut prompt = String::from(
            "You are a browser automation vision assistant. \
             Analyze the provided screenshot and describe what you see on the page.",
        );
        if !config.react_to.is_empty() {
            prompt.push_str(&format!(
                " Pay special attention to: {}.",
                config.react_to.join(", ")
            ));
        }
        if config.describe {
            prompt.push_str(
                " Provide a detailed description of the page layout, content, and interactive elements.",
            );
        }
        prompt.push_str(
            " If you detect actionable items (errors, captchas, popups), suggest concrete actions.",
        );
        prompt
    });

    let mut user_parts = vec![VisionContent::ImageUrl {
        image_url: ImageUrl {
            url: format!("data:{mime};base64,{frame_base64}"),
            detail: Some("auto".to_string()),
        },
    }];

    let user_text = additional_prompt
        .unwrap_or("Describe what you see in this browser screenshot.");
    user_parts.push(VisionContent::Text {
        text: user_text.to_string(),
    });

    VisionRequest {
        model: config.model.clone(),
        messages: vec![
            VisionMessage {
                role: "system".to_string(),
                content: vec![VisionContent::Text {
                    text: system_prompt,
                }],
            },
            VisionMessage {
                role: "user".to_string(),
                content: user_parts,
            },
        ],
        max_tokens: config.max_tokens,
    }
}

fn extract_suggested_actions(text: &str) -> Vec<SuggestedAction> {
    let mut actions = Vec::new();
    let patterns: &[(&str, &str)] = &[
        ("click", "click"),
        ("dismiss", "dismiss"),
        ("close", "dismiss"),
        ("solve", "solve_captcha"),
        ("wait", "wait"),
        ("navigate", "navigate"),
        ("scroll", "scroll"),
    ];
    for (keyword, action_type) in patterns {
        if text.contains(keyword) {
            actions.push(SuggestedAction {
                action_type: action_type.to_string(),
                target: None,
                reasoning: format!("Model response mentions '{keyword}'"),
            });
        }
    }
    actions
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", now.as_secs(), now.subsec_millis())
}

fn append_log(path: &str, line: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_valid_fps() {
        let cfg = VisionConfig::default();
        assert!(cfg.fps > 0.0);
        assert!(cfg.fps <= cfg.max_fps);
    }

    #[test]
    fn screenshot_format_default_is_jpeg() {
        let fmt = ScreenshotFormat::default();
        matches!(fmt, ScreenshotFormat::Jpeg { quality: 70 });
    }

    #[test]
    fn prepare_request_builds_valid_payload() {
        let config = VisionConfig::default();
        let req = build_vision_request(&config, "abc123", None);
        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(req.messages[1].role, "user");
    }

    #[test]
    fn prepare_request_with_custom_prompt() {
        let config = VisionConfig {
            prompt: Some("Custom prompt".to_string()),
            ..Default::default()
        };
        let req = build_vision_request(&config, "data", Some("Extra"));
        let sys_text = match &req.messages[0].content[0] {
            VisionContent::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert_eq!(sys_text, "Custom prompt");
    }

    #[test]
    fn parse_freeform_response_detects_keywords() {
        let stream = VisionStream::new(VisionConfig::default());
        let obs = stream
            .parse_vision_response("I see an error dialog and a captcha on the page", 1)
            .expect("should parse");
        assert!(!obs.detected.is_empty());
        let categories: Vec<&str> = obs.detected.iter().map(|d| d.category.as_str()).collect();
        assert!(categories.contains(&"error_dialog"));
        assert!(categories.contains(&"captcha"));
    }

    #[test]
    fn parse_structured_response() {
        let obs_json = serde_json::json!({
            "timestamp": "1234",
            "frame_index": 5,
            "description": "test",
            "detected": [],
            "suggested_actions": [],
            "confidence": 0.9,
            "tokens_used": 100,
            "cost_cents": 1
        });
        let stream = VisionStream::new(VisionConfig::default());
        let obs = stream
            .parse_vision_response(&serde_json::to_string(&obs_json).unwrap(), 5)
            .expect("should parse");
        assert_eq!(obs.frame_index, 5);
        assert_eq!(obs.tokens_used, 100);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let cfg = VisionConfig {
            model: "claude-sonnet".to_string(),
            fps: 1.0,
            describe: true,
            react_to: vec!["errors".to_string(), "captchas".to_string()],
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let deserialized: VisionConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.model, "claude-sonnet");
        assert_eq!(deserialized.react_to.len(), 2);
    }

    #[test]
    fn extract_suggested_actions_works() {
        let actions = extract_suggested_actions("you should click the button and wait for the page");
        let types: Vec<&str> = actions.iter().map(|a| a.action_type.as_str()).collect();
        assert!(types.contains(&"click"));
        assert!(types.contains(&"wait"));
    }

    #[tokio::test]
    async fn set_fps_validates_bounds() {
        let stream = VisionStream::new(VisionConfig::default());
        assert!(stream.set_fps(0.0).await.is_err());
        assert!(stream.set_fps(-1.0).await.is_err());
        assert!(stream.set_fps(100.0).await.is_err());
        assert!(stream.set_fps(1.0).await.is_ok());
    }

    #[tokio::test]
    async fn record_observation_updates_stats() {
        let stream = VisionStream::new(VisionConfig::default());
        let obs = VisionObservation {
            timestamp: "0".to_string(),
            frame_index: 1,
            description: "test".to_string(),
            detected: vec![Detection {
                category: "error_dialog".to_string(),
                description: "err".to_string(),
                confidence: 0.9,
                bounding_box: None,
            }],
            suggested_actions: vec![],
            confidence: 0.9,
            tokens_used: 50,
            cost_cents: 2,
        };
        stream.record_observation(obs).await;
        let status = stream.status().await;
        assert_eq!(status.observations_made, 1);
        assert_eq!(status.total_tokens, 50);
        assert_eq!(status.total_cost_cents, 2);
        assert_eq!(*status.detections_by_category.get("error_dialog").unwrap_or(&0), 1);
    }
}
