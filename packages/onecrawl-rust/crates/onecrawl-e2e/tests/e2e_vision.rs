//! E2E tests for the vision stream.
//! Tests VisionConfig, VisionStream status, observations, and FPS settings.

use onecrawl_cdp::vision::{ScreenshotFormat, VisionConfig, VisionStream};

fn test_vision_config() -> VisionConfig {
    VisionConfig {
        model: "test-model".to_string(),
        api_key: None,
        api_url: None,
        fps: 1.0,
        max_fps: 2.0,
        describe: false,
        react_to: vec![],
        prompt: None,
        max_tokens: Some(500),
        max_cost_cents: None,
        screenshot_format: ScreenshotFormat::Png,
        resolution: None,
        output_log: None,
    }
}

// ────────────────────── Construction ──────────────────────

#[test]
fn e2e_vision_config_construction() {
    let config = test_vision_config();
    assert_eq!(config.model, "test-model");
    assert_eq!(config.fps, 1.0);
}

#[test]
fn e2e_vision_stream_new() {
    let stream = VisionStream::new(test_vision_config());
    let _ = stream;
}

// ────────────────────── status ──────────────────────

#[tokio::test]
async fn e2e_vision_status_initial() {
    let stream = VisionStream::new(test_vision_config());
    let status = stream.status().await;
    assert!(!status.running);
    assert_eq!(status.frames_captured, 0);
    assert_eq!(status.observations_made, 0);
    assert_eq!(status.total_tokens, 0);
}

// ────────────────────── observations ──────────────────────

#[tokio::test]
async fn e2e_vision_observations_empty() {
    let stream = VisionStream::new(test_vision_config());
    let obs = stream.observations(10).await;
    assert!(obs.is_empty());
}

// ────────────────────── set_fps ──────────────────────

#[tokio::test]
async fn e2e_vision_set_fps_valid() {
    let stream = VisionStream::new(test_vision_config());
    stream.set_fps(0.5).await.unwrap();

    let status = stream.status().await;
    assert!((status.fps - 0.5).abs() < f32::EPSILON);
}

#[tokio::test]
async fn e2e_vision_set_fps_zero_fails() {
    let stream = VisionStream::new(test_vision_config());
    let result = stream.set_fps(0.0).await;
    assert!(result.is_err(), "FPS of 0.0 should be rejected");
}

#[tokio::test]
async fn e2e_vision_set_fps_nan_fails() {
    let stream = VisionStream::new(test_vision_config());
    let result = stream.set_fps(f32::NAN).await;
    assert!(result.is_err(), "NaN FPS should be rejected");
}

#[tokio::test]
async fn e2e_vision_set_fps_negative_fails() {
    let stream = VisionStream::new(test_vision_config());
    let result = stream.set_fps(-1.0).await;
    assert!(result.is_err(), "negative FPS should be rejected");
}
