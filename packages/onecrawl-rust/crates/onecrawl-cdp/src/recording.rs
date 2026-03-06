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
