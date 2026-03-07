use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AgentCliAction {
    /// Autonomous observe→plan→act→verify loop
    Loop {
        /// Goal description
        goal: String,
        /// Max observation steps
        #[arg(long, default_value = "10")]
        max_steps: usize,
        /// JS expression that returns "true" when goal is met
        #[arg(long)]
        verify: Option<String>,
    },
    /// Semantic goal verification with assertions
    GoalAssert {
        /// Assertion type (url_contains, title_contains, element_exists, text_contains, element_visible)
        #[arg(long, short = 't')]
        assertion_type: String,
        /// Value to check
        value: String,
    },
    /// Get annotated page observation with element coordinates
    Observe,
    /// Manage session context (set/get/get_all/clear)
    Context {
        /// Command: set, get, get_all, clear
        command: String,
        /// Key (for set/get)
        #[arg(long)]
        key: Option<String>,
        /// Value (for set)
        #[arg(long)]
        value: Option<String>,
    },
    /// Execute JS action chain with error recovery
    Chain {
        /// JS expressions to execute in sequence
        #[arg(required = true)]
        actions: Vec<String>,
        /// Error handling: retry, skip, abort (default: skip)
        #[arg(long, default_value = "skip")]
        on_error: String,
        /// Max retries per action
        #[arg(long, default_value = "2")]
        retries: usize,
    },
    /// Analyze page and recommend next actions
    Think,
}
