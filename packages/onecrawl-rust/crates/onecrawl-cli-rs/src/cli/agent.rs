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
    /// Autonomous goal-based browser automation
    Auto {
        /// Natural language goal
        #[arg(long)]
        goal: Option<String>,
        /// LLM model name (for reference)
        #[arg(long)]
        model: Option<String>,
        /// Max steps (default 50)
        #[arg(long, default_value = "50")]
        max_steps: u32,
        /// Cost cap in dollars (e.g. 0.50)
        #[arg(long)]
        max_cost: Option<f64>,
        /// Capture screenshot after each step
        #[arg(long)]
        screenshot_every_step: bool,
        /// Output file path (e.g. results.csv)
        #[arg(long)]
        output: Option<String>,
        /// Output format: csv, json, jsonl
        #[arg(long)]
        output_format: Option<String>,
        /// Enable verbose logging
        #[arg(long)]
        verbose: bool,
        /// Overall timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
        /// Resume from saved state file
        #[arg(long)]
        resume: Option<String>,
        /// Save state path for resume
        #[arg(long)]
        save_state: Option<String>,
    },
    /// Plan steps for a goal without executing
    Plan {
        /// Natural language goal
        #[arg(long)]
        goal: String,
        /// Enable verbose output
        #[arg(long)]
        verbose: bool,
    },
    /// Get status of running agent
    Status,
    /// Stop a running agent
    Stop {
        /// Save state path for resume
        #[arg(long)]
        save_state: Option<String>,
    },
    /// Get result of the last completed run
    Result,
}
