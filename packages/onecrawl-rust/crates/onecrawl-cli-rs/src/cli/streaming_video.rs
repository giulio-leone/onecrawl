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
}
