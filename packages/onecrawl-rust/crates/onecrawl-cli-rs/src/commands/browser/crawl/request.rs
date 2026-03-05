use super::super::helpers::{with_page};

// Rate Limiter (standalone — no Page required)
// Retry Queue (standalone — no Page required)
// Task Scheduler (standalone — no Page required)
// Session Pool (standalone — no Page required)

pub async fn request_execute(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let req: onecrawl_cdp::QueuedRequest =
            serde_json::from_str(&json).map_err(|e| format!("Invalid request JSON: {e}"))?;
        let result = onecrawl_cdp::request_queue::execute_request(&page, &req)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn request_batch(json: &str, concurrency: usize, delay: u64) {
    let json = json.to_string();
    with_page(|page| async move {
        let reqs: Vec<onecrawl_cdp::QueuedRequest> =
            serde_json::from_str(&json).map_err(|e| format!("Invalid requests JSON: {e}"))?;
        let config = onecrawl_cdp::QueueConfig {
            concurrency,
            delay_between_ms: delay,
            ..Default::default()
        };
        let results = onecrawl_cdp::request_queue::execute_batch(&page, &reqs, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

