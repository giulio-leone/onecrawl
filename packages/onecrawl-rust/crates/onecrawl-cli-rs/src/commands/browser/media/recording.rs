use colored::Colorize;
use std::path::PathBuf;
use std::sync::OnceLock;

use super::super::helpers::with_page;

static RECORDING: OnceLock<onecrawl_cdp::SharedRecording> = OnceLock::new();

fn shared_recording() -> &'static onecrawl_cdp::SharedRecording {
    RECORDING.get_or_init(onecrawl_cdp::new_shared_recording)
}

pub async fn recording_start(output: &str, fps: u32) {
    let rec = shared_recording().clone();
    let mut guard = rec.lock().await;
    if guard.as_ref().is_some_and(|r| r.is_recording()) {
        eprintln!("{} Recording already in progress", "✗".red());
        return;
    }
    let state = onecrawl_cdp::RecordingState::new(PathBuf::from(output), fps);
    *guard = Some(state);
    guard.as_mut().unwrap().start();
    drop(guard);

    // Start screencast for frame delivery
    with_page(|page| async move {
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: "jpeg".to_string(),
            quality: Some(60),
            max_width: Some(1280),
            max_height: Some(720),
            every_nth_frame: Some(1),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Recording started → {output} ({fps} fps)",
            "✓".green(),
        );
        Ok(())
    })
    .await;
}

pub async fn recording_stop() {
    let rec = shared_recording().clone();

    // Stop screencast first
    with_page(|page| async move {
        let _ = onecrawl_cdp::screencast::stop_screencast(&page).await;

        // Capture a final batch of frames if none were collected via events
        let mut guard = rec.lock().await;
        if let Some(state) = guard.as_mut() {
            if state.is_recording() && state.frame_count() == 0 {
                // Capture at least one frame as a snapshot
                let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
                if let Ok(bytes) = onecrawl_cdp::screencast::capture_frame(&page, &opts).await {
                    state.add_frame(bytes);
                }
            }
            state.stop();
            match state.save_frames() {
                Ok(dir) => {
                    println!(
                        "{} Recording saved: {} frames → {}",
                        "✓".green(),
                        state.frame_count(),
                        dir.display()
                    );
                }
                Err(e) => {
                    eprintln!("{} Failed to save frames: {e}", "✗".red());
                }
            }
        } else {
            eprintln!("{} No recording in progress", "✗".red());
        }
        Ok(())
    })
    .await;
}

pub async fn recording_status() {
    let rec = shared_recording().clone();
    let guard = rec.lock().await;
    match guard.as_ref() {
        Some(state) => {
            let status = if state.is_recording() {
                "recording"
            } else {
                "stopped"
            };
            println!(
                "{{\"status\":\"{status}\",\"frames\":{},\"fps\":{},\"output\":\"{}\"}}",
                state.frame_count(),
                state.fps(),
                state.output_path().display()
            );
        }
        None => {
            println!("{{\"status\":\"idle\",\"frames\":0}}");
        }
    }
}

pub async fn video_encode(frames_dir: &str, output: &str, fps: u32, format: &str) {
    match onecrawl_cdp::recording::encode_video(frames_dir, output, fps, format) {
        Ok(result) => println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default()),
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn video_record(duration: u64, output: &str, fps: u32, format: &str) {
    with_page(|page| async move {
        let total_frames = (fps as u64 * duration) as usize;
        let interval_ms = 1000 / fps as u64;
        let dir = "/tmp/onecrawl-recording";
        let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
        let stream = onecrawl_cdp::screencast::stream_to_disk(
            &page, &opts, dir, total_frames, interval_ms,
        )
        .await
        .map_err(|e| e.to_string())?;
        println!(
            "{} Captured {} frames",
            "✓".green(),
            stream.frames_captured
        );
        match onecrawl_cdp::recording::encode_video(dir, output, fps, format) {
            Ok(video) => println!("{}", serde_json::to_string_pretty(&video).unwrap_or_default()),
            Err(e) => println!("Encoding failed (ffmpeg required): {e}"),
        }
        Ok(())
    })
    .await;
}
