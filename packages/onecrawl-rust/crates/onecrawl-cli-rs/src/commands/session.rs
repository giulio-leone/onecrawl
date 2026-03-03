use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionAction {
    /// Start a new browser session
    Start {
        /// Headless mode
        #[arg(short = 'H', long)]
        headless: bool,
        /// Connect to existing browser via CDP URL
        #[arg(short, long)]
        connect: Option<String>,
    },
    /// Show session info
    Info,
    /// Close the current session
    Close,
}
