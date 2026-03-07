use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum EventsAction {
    /// Start event bus HTTP listener
    Listen {
        /// Port to listen on
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Emit an event to the bus
    Emit {
        /// Event type (e.g. 'page:loaded', 'user:action')
        event_type: String,
        /// JSON data payload
        #[arg(long)]
        data: Option<String>,
        /// Event source name
        #[arg(long, default_value = "cli")]
        source: String,
    },
    /// Subscribe a webhook to an event pattern
    Subscribe {
        /// Glob pattern for event types (e.g. 'page:*', '**')
        event_pattern: String,
        /// Webhook URL to POST events to
        #[arg(long)]
        webhook: String,
        /// HMAC-SHA256 signing secret
        #[arg(long)]
        secret: Option<String>,
    },
    /// Remove a webhook subscription
    Unsubscribe {
        /// Subscription ID to remove
        subscription_id: String,
    },
    /// List all webhook subscriptions
    List,
    /// Get recent events from journal
    Recent {
        /// Max events to return
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Replay events matching a pattern
    Replay {
        /// Glob pattern for event types
        event_pattern: String,
        /// Only replay events after this ISO 8601 timestamp
        #[arg(long)]
        since: Option<String>,
    },
    /// Get event bus statistics
    Stats,
    /// Clear the event journal
    Clear,
}
