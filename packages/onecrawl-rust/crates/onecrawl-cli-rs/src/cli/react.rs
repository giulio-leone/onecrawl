use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum ReactAction {
    /// Start an event reactor with inline rules
    Start {
        /// Event type to react to (dom_mutation, console, page_error, navigation, network_request, network_response, websocket, timer)
        #[arg(long)]
        on: String,
        /// CSS selector filter (for DOM mutations)
        #[arg(long)]
        selector: Option<String>,
        /// URL glob pattern (for network events)
        #[arg(long)]
        url: Option<String>,
        /// Handler type (log, screenshot, evaluate, webhook, store)
        #[arg(long)]
        handler: String,
        /// JavaScript to evaluate (for evaluate handler)
        #[arg(long)]
        script: Option<String>,
        /// AI prompt (for ai_respond handler)
        #[arg(long)]
        prompt: Option<String>,
        /// AI model (for ai_respond handler)
        #[arg(long)]
        model: Option<String>,
        /// File path for store/log output
        #[arg(long)]
        output: Option<String>,
        /// Reactor name
        #[arg(long, default_value = "default")]
        name: String,
        /// Max events per minute (rate limit)
        #[arg(long)]
        max_epm: Option<u32>,
    },
    /// Stop a running reactor
    Stop {
        /// Reactor name
        #[arg(long, default_value = "default")]
        name: String,
    },
    /// Get reactor status
    Status {
        /// Reactor name
        #[arg(long, default_value = "default")]
        name: String,
    },
    /// Add a rule to a running reactor
    AddRule {
        /// Rule ID
        #[arg(long)]
        id: String,
        /// Event type to react to
        #[arg(long)]
        on: String,
        /// Handler type (log, screenshot, evaluate, webhook, store)
        #[arg(long)]
        handler: String,
        /// CSS selector filter
        #[arg(long)]
        selector: Option<String>,
        /// URL glob pattern
        #[arg(long)]
        url: Option<String>,
        /// Message substring filter
        #[arg(long)]
        message: Option<String>,
        /// JavaScript to evaluate
        #[arg(long)]
        script: Option<String>,
        /// File path for output
        #[arg(long)]
        output: Option<String>,
    },
    /// Remove a rule by ID
    RemoveRule {
        /// Rule ID to remove
        #[arg(long)]
        id: String,
    },
    /// List all reactor rules
    ListRules {
        /// Reactor name
        #[arg(long, default_value = "default")]
        name: String,
    },
    /// Get recent matched events
    Events {
        /// Max events to return
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}
