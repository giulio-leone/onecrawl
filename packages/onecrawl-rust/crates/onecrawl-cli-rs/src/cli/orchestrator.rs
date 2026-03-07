use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum OrchestrateAction {
    /// Run a multi-device orchestration from a JSON workflow file
    Run {
        /// Path to orchestration JSON file
        file: String,
        /// Enable verbose output
        #[arg(long)]
        verbose: bool,
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Validate an orchestration JSON file without executing
    Validate {
        /// Path to orchestration JSON file
        file: String,
    },
    /// List connected devices and their status
    Devices,
    /// Stop a running orchestration
    Stop,
}
