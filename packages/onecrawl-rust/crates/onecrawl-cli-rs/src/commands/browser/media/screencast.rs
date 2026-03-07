use colored::Colorize;

use super::super::helpers::with_page;

pub async fn stream_start(width: u32, height: u32, format: &str, quality: u32) {
    with_page(|page| async move {
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: format.to_string(),
            quality: Some(quality),
            max_width: Some(width),
            max_height: Some(height),
            every_nth_frame: Some(1),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Screencast started ({}×{}, {format}, q={quality})",
            "✓".green(),
            width,
            height
        );
        Ok(())
    })
    .await;
}

pub async fn stream_stop() {
    with_page(|page| async move {
        onecrawl_cdp::screencast::stop_screencast(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Screencast stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn stream_frame(output: &str) {
    with_page(|page| async move {
        let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
        let bytes = onecrawl_cdp::screencast::capture_frame(&page, &opts)
            .await
            .map_err(|e| e.to_string())?;
        std::fs::write(output, &bytes).map_err(|e| e.to_string())?;
        println!(
            "{} Frame captured → {} ({} bytes)",
            "✓".green(),
            output,
            bytes.len()
        );
        Ok(())
    })
    .await;
}

pub async fn stream_capture(output_dir: &str, count: usize, interval_ms: u64) {
    with_page(|page| async move {
        let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
        let result = onecrawl_cdp::screencast::stream_to_disk(
            &page, &opts, output_dir, count, interval_ms,
        )
        .await
        .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
        Ok(())
    })
    .await;
}
