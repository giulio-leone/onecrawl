use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum VisionAction {
    /// Start streaming AI vision
    Start {
        /// Model identifier (e.g. gpt-4o, gemini-2.5-pro, claude-sonnet)
        #[arg(long, default_value = "gpt-4o")]
        model: String,
        /// Frames per second to capture
        #[arg(long, default_value = "0.5")]
        fps: f32,
        /// Continuously describe what's on screen
        #[arg(long)]
        describe: bool,
        /// What to react to (comma-separated: errors,captchas,popups,changes)
        #[arg(long)]
        react_to: Option<String>,
        /// Path to log descriptions
        #[arg(short, long)]
        output: Option<String>,
        /// Custom system prompt
        #[arg(long)]
        prompt: Option<String>,
        /// Max tokens per response
        #[arg(long)]
        max_tokens: Option<u32>,
        /// Cost cap in cents
        #[arg(long)]
        max_cost_cents: Option<u32>,
        /// Screenshot format: jpeg or png
        #[arg(long, default_value = "jpeg")]
        format: String,
        /// JPEG quality 0-100
        #[arg(long, default_value = "70")]
        quality: u8,
    },
    /// Stop vision stream
    Stop,
    /// Get vision stream status
    Status,
    /// One-shot describe current page
    Describe,
    /// Get recent vision observations
    Observations {
        /// Maximum number of observations to return
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Update capture FPS
    SetFps {
        /// New frames per second
        fps: f32,
    },
}
