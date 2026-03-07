//! Video recording — captures screencast frames and saves them as image sequences.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// State for an active recording session.
pub struct RecordingState {
    frames: Vec<Vec<u8>>,
    format: String,
    fps: u32,
    output_path: PathBuf,
    is_recording: bool,
}

impl RecordingState {
    pub fn new(output_path: PathBuf, fps: u32) -> Self {
        let format = output_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpeg")
            .to_string();
        Self {
            frames: Vec::new(),
            format,
            fps,
            output_path,
            is_recording: false,
        }
    }

    pub fn add_frame(&mut self, data: Vec<u8>) {
        if self.is_recording {
            self.frames.push(data);
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn start(&mut self) {
        self.is_recording = true;
    }

    pub fn stop(&mut self) {
        self.is_recording = false;
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    /// Save frames as individual images (frame_0001.jpg, frame_0002.jpg, …).
    /// Returns the directory path where frames were saved.
    pub fn save_frames(&self) -> Result<PathBuf, String> {
        let dir = self.output_path.parent().unwrap_or(Path::new("."));
        let stem = self
            .output_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("recording");
        let frames_dir = dir.join(format!("{stem}_frames"));
        std::fs::create_dir_all(&frames_dir).map_err(|e| e.to_string())?;

        let ext = if self.format == "png" { "png" } else { "jpg" };
        for (i, frame) in self.frames.iter().enumerate() {
            let filename = frames_dir.join(format!("frame_{:04}.{ext}", i + 1));
            std::fs::write(&filename, frame).map_err(|e| e.to_string())?;
        }
        Ok(frames_dir)
    }
}

/// Thread-safe recording handle.
pub type SharedRecording = Arc<Mutex<Option<RecordingState>>>;

pub fn new_shared_recording() -> SharedRecording {
    Arc::new(Mutex::new(None))
}

// ────────────── Video Encoding (ffmpeg) ──────────────

use serde::{Deserialize, Serialize};
use std::process::Command;

/// Result of video encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoResult {
    pub output_path: String,
    pub frames_used: usize,
    pub format: String,
    pub duration_ms: u64,
    pub file_size: u64,
}

/// Encode saved frames into a video file using ffmpeg.
/// Frames must be saved as frame_0001.jpg, frame_0002.jpg, etc. in `frames_dir`.
pub fn encode_video(
    frames_dir: &str,
    output_path: &str,
    fps: u32,
    format: &str,
) -> std::result::Result<VideoResult, String> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err("ffmpeg not found — install ffmpeg for video encoding".to_string());
    }

    let start = std::time::Instant::now();

    let frame_count = std::fs::read_dir(frames_dir)
        .map_err(|e| format!("read dir: {e}"))?
        .filter(|entry| {
            entry.as_ref().map_or(false, |e| {
                e.path()
                    .extension()
                    .map_or(false, |x| x == "jpg" || x == "png")
            })
        })
        .count();

    if frame_count == 0 {
        return Err("no frames found in directory".to_string());
    }

    // Detect extension used in the directory
    let has_png = std::fs::read_dir(frames_dir)
        .map_err(|e| format!("read dir: {e}"))?
        .any(|e| e.map_or(false, |e| e.path().extension().map_or(false, |x| x == "png")));
    let ext = if has_png { "png" } else { "jpg" };
    let input_pattern = format!("{frames_dir}/frame_%04d.{ext}");

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-framerate")
        .arg(fps.to_string())
        .arg("-i")
        .arg(&input_pattern);

    match format {
        "webm" => {
            cmd.arg("-c:v")
                .arg("libvpx-vp9")
                .arg("-b:v")
                .arg("1M")
                .arg("-pix_fmt")
                .arg("yuv420p");
        }
        "gif" => {
            cmd.arg("-vf")
                .arg(format!("fps={fps},scale=800:-1:flags=lanczos"));
        }
        _ => {
            // mp4 or fallback
            cmd.arg("-c:v")
                .arg("libx264")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-crf")
                .arg("23");
        }
    }

    cmd.arg(output_path);

    let output = cmd.output().map_err(|e| format!("ffmpeg exec: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "ffmpeg failed: {}",
            stderr.chars().take(500).collect::<String>()
        ));
    }

    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    Ok(VideoResult {
        output_path: output_path.to_string(),
        frames_used: frame_count,
        format: format.to_string(),
        duration_ms: start.elapsed().as_millis() as u64,
        file_size,
    })
}

/// Save raw byte frames to disk, then encode as video.
pub fn save_and_encode(
    frames: &[Vec<u8>],
    output_dir: &str,
    output_path: &str,
    fps: u32,
    format: &str,
) -> std::result::Result<VideoResult, String> {
    std::fs::create_dir_all(output_dir).map_err(|e| format!("mkdir: {e}"))?;

    for (i, frame_bytes) in frames.iter().enumerate() {
        let path = format!("{output_dir}/frame_{:04}.jpg", i + 1);
        std::fs::write(&path, frame_bytes).map_err(|e| format!("write frame {i}: {e}"))?;
    }

    encode_video(output_dir, output_path, fps, format)
}
