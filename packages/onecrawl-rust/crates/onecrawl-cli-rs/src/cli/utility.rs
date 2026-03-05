use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum RateLimitAction {
    /// Set rate limiter config (preset or JSON)
    Set {
        /// Preset name: conservative, moderate, aggressive, unlimited
        #[arg(long)]
        preset: Option<String>,
        /// JSON config string (alternative to preset)
        #[arg(long)]
        config: Option<String>,
    },
    /// Show rate limiter statistics
    Stats,
    /// Reset rate limiter counters
    Reset,
}


#[derive(Subcommand)]
pub(crate) enum RetryAction {
    /// Enqueue a URL/operation for retry
    Enqueue {
        /// Target URL
        url: String,
        /// Operation type (navigate, click, extract, submit, etc.)
        operation: String,
        /// Optional payload
        #[arg(long)]
        payload: Option<String>,
    },
    /// Get the next item due for retry
    Next,
    /// Mark an item as successful
    Success {
        /// Item id
        id: String,
    },
    /// Mark an item as failed
    Fail {
        /// Item id
        id: String,
        /// Error message
        error: String,
    },
    /// Show retry queue statistics
    Stats,
    /// Clear completed items
    Clear,
    /// Save queue to a file
    Save {
        /// Output file path (JSON)
        path: String,
    },
    /// Load queue from a file
    Load {
        /// Input file path (JSON)
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum ScheduleAction {
    /// Add a new scheduled task
    Add {
        /// Task name
        name: String,
        /// Task type (navigate, extract, screenshot, crawl, custom)
        #[arg(short = 't', long)]
        task_type: String,
        /// JSON config for the task
        #[arg(short, long, default_value = "{}")]
        config: String,
        /// Interval in ms (0 = one-shot)
        #[arg(short, long, default_value = "0")]
        interval: u64,
        /// Initial delay in ms
        #[arg(short, long, default_value = "0")]
        delay: u64,
        /// Maximum number of runs
        #[arg(short, long)]
        max_runs: Option<usize>,
    },
    /// Remove a task by ID
    Remove {
        /// Task ID
        id: String,
    },
    /// Pause a task
    Pause {
        /// Task ID
        id: String,
    },
    /// Resume a paused task
    Resume {
        /// Task ID
        id: String,
    },
    /// List all tasks
    List,
    /// Show scheduler statistics
    Stats,
    /// Save scheduler to file
    Save {
        /// Output path
        path: String,
    },
    /// Load scheduler from file
    Load {
        /// Input path
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum PoolAction {
    /// Add a session to the pool
    Add {
        /// Session name
        name: String,
        /// Tags
        #[arg(short, long)]
        tags: Vec<String>,
    },
    /// Get next available session
    Next,
    /// Show pool statistics
    Stats,
    /// Clean up idle sessions
    Cleanup,
    /// Save pool to file
    Save {
        /// Output path
        path: String,
    },
    /// Load pool from file
    Load {
        /// Input path
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum BenchAction {
    /// Run CDP benchmark suite
    Run {
        /// Number of iterations per benchmark
        #[arg(short, long, default_value = "20")]
        iterations: u32,
        /// Filter to specific module
        #[arg(short, long)]
        module: Option<String>,
    },
    /// Show last benchmark results
    Report {
        /// Output format: table or json
        #[arg(short, long, default_value = "table")]
        format: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum GeoAction {
    /// Apply a geo profile by preset name or JSON
    Apply {
        /// Preset name (e.g. "new york") or JSON GeoProfile
        profile: String,
    },
    /// List available geo presets
    Presets,
    /// Get current geolocation as seen by the page
    Current,
}


#[derive(Subcommand)]
pub(crate) enum RequestAction {
    /// Execute a single request (JSON QueuedRequest)
    Execute {
        /// JSON QueuedRequest
        json: String,
    },
    /// Execute a batch of requests (JSON array)
    Batch {
        /// JSON array of QueuedRequest
        json: String,
        /// Concurrency limit
        #[arg(short, long, default_value = "3")]
        concurrency: usize,
        /// Delay between requests in ms
        #[arg(short, long, default_value = "100")]
        delay: u64,
    },
}

