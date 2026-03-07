use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ComputerCliAction {
    /// Take screenshot with numbered element overlays
    AnnotatedScreenshot {
        /// Output file path
        #[arg(default_value = "annotated.png")]
        output: String,
    },
    /// Try action with alternative strategies on failure
    AdaptiveRetry {
        /// Primary JS action
        action: String,
        /// Alternative JS strategies
        #[arg(long)]
        alt: Vec<String>,
        /// Max retries
        #[arg(long, default_value = "3")]
        retries: usize,
    },
    /// Click at specific viewport coordinates
    ClickAt {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },
    /// Get synchronized state from all tabs
    MultiPageSync,
    /// Replay a sequence of input events from JSON
    InputReplay {
        /// Path to JSON file with event sequence
        events_file: String,
    },
}
