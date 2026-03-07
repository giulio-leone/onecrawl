use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum HarnessAction {
    /// Check browser health and responsiveness
    HealthCheck,
    /// Circuit breaker control
    CircuitBreaker {
        /// Command: status, record_success, record_failure, reset
        command: String,
        /// Error message (for record_failure)
        #[arg(long)]
        error: Option<String>,
    },
    /// Test CDP connection with retry
    ReconnectCdp {
        /// Max retries
        #[arg(long, default_value = "5")]
        retries: usize,
    },
    /// Tab garbage collection info
    GcTabs,
    /// Watchdog: check for crashes/hangs
    Watchdog,
}
