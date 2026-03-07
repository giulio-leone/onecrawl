pub async fn handle(action: crate::cli::VisionAction) {
    use crate::cli::VisionAction;
    match action {
        VisionAction::Start {
            model,
            fps,
            describe,
            react_to,
            output,
            prompt,
            max_tokens,
            max_cost_cents,
            format,
            quality,
        } => {
            let react_to_vec: Vec<String> = react_to
                .map(|r: String| r.split(',').map(|s: &str| s.trim().to_string()).collect::<Vec<_>>())
                .unwrap_or_default();

            let params = serde_json::json!({
                "model": model,
                "fps": fps,
                "describe": describe,
                "react_to": react_to_vec,
                "output": output,
                "prompt": prompt,
                "max_tokens": max_tokens,
                "max_cost_cents": max_cost_cents,
                "format": format,
                "quality": quality,
            });
            run_vision("vision_stream_start", params).await;
        }
        VisionAction::Stop => {
            run_vision("vision_stream_stop", serde_json::json!({})).await;
        }
        VisionAction::Status => {
            run_vision("vision_stream_status", serde_json::json!({})).await;
        }
        VisionAction::Describe => {
            run_vision("vision_stream_describe", serde_json::json!({})).await;
        }
        VisionAction::Observations { limit } => {
            run_vision(
                "vision_stream_observations",
                serde_json::json!({ "limit": limit }),
            )
            .await;
        }
        VisionAction::SetFps { fps } => {
            run_vision(
                "vision_stream_set_fps",
                serde_json::json!({ "fps": fps }),
            )
            .await;
        }
    }
}

async fn run_vision(action: &str, params: serde_json::Value) {
    super::browser::run::run_action("agent", action, Some(&serde_json::to_string(&params).unwrap_or_default())).await;
}
