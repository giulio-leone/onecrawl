use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum StreamAction {
    /// Start live screencast
    Start {
        #[arg(long, default_value = "1280")]
        width: u32,
        #[arg(long, default_value = "720")]
        height: u32,
        #[arg(long, default_value = "jpeg")]
        format: String,
        #[arg(long, default_value = "60")]
        quality: u32,
    },
    /// Stop screencast
    Stop,
    /// Capture a single frame
    Frame {
        #[arg(short, long)]
        output: String,
    },
    /// Capture a burst of frames to disk
    Capture {
        /// Output directory
        #[arg(short, long, default_value = "/tmp/onecrawl-stream")]
        output: String,
        /// Number of frames
        #[arg(short, long, default_value = "30")]
        count: usize,
        /// Interval between frames in ms
        #[arg(short, long, default_value = "200")]
        interval: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum RecordAction {
    /// Start recording
    Start {
        #[arg(short, long, default_value = "recording.webm")]
        output: String,
        #[arg(long, default_value = "5")]
        fps: u32,
    },
    /// Stop recording and save frames
    Stop,
    /// Get recording status
    Status,
    /// Encode frames directory into video (requires ffmpeg)
    Encode {
        /// Directory containing frame_NNNN.jpg files
        frames_dir: String,
        /// Output video path
        #[arg(short, long, default_value = "output.mp4")]
        output: String,
        /// Frames per second
        #[arg(long, default_value = "5")]
        fps: u32,
        /// Video format (mp4, webm, gif)
        #[arg(short, long, default_value = "mp4")]
        format: String,
    },
    /// Record browser video: capture + encode (requires ffmpeg)
    Video {
        /// Recording duration in seconds
        #[arg(short, long, default_value = "5")]
        duration: u64,
        /// Output video path
        #[arg(short, long, default_value = "recording.mp4")]
        output: String,
        /// Frames per second
        #[arg(long, default_value = "5")]
        fps: u32,
        /// Video format (mp4, webm, gif)
        #[arg(short, long, default_value = "mp4")]
        format: String,
    },
}
