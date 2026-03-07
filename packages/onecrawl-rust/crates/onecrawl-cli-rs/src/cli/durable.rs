use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum DurableAction {
    /// Start a new durable browser session
    Start {
        /// Unique name for the durable session
        #[arg(long)]
        name: String,
        /// Checkpoint interval (e.g. 30s, 5m)
        #[arg(long, default_value = "30s")]
        checkpoint_interval: String,
        /// State directory path
        #[arg(long)]
        persist_state: Option<String>,
        /// Enable auto-reconnect on crash
        #[arg(long, default_value_t = true)]
        auto_reconnect: bool,
        /// Maximum uptime (e.g. 1h, 7d)
        #[arg(long)]
        max_uptime: Option<String>,
        /// Crash policy: restart, stop, or notify
        #[arg(long, default_value = "restart")]
        on_crash: String,
    },
    /// Gracefully stop a durable session
    Stop {
        /// Name of the durable session
        #[arg(long)]
        name: String,
    },
    /// Force an immediate checkpoint
    Checkpoint {
        /// Name of the durable session
        #[arg(long)]
        name: String,
    },
    /// Restore from a saved checkpoint
    Restore {
        /// Name of the durable session
        #[arg(long)]
        name: String,
    },
    /// Get status of a durable session
    Status {
        /// Name of the durable session (omit for default)
        #[arg(long)]
        name: Option<String>,
    },
    /// List all saved durable sessions
    List,
    /// Delete a saved session state
    Delete {
        /// Name of the durable session
        #[arg(long)]
        name: String,
    },
}
