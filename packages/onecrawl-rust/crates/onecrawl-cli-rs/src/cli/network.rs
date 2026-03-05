use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum DomainAction {
    /// Block specific domains
    Block {
        /// Domains to block
        domains: Vec<String>,
    },
    /// Block an entire category (ads, trackers, social, fonts, media)
    BlockCategory {
        /// Category name
        category: String,
    },
    /// Remove all domain blocks
    Unblock,
    /// Show blocking statistics
    Stats,
    /// List currently blocked domains
    List,
    /// Show available block categories
    Categories,
}


#[derive(Subcommand)]
pub(crate) enum HttpAction {
    /// HTTP GET request
    Get {
        /// URL to fetch
        url: String,
    },
    /// HTTP POST request
    Post {
        /// URL to post to
        url: String,
        /// Request body
        #[arg(long)]
        body: String,
        /// Content-Type header
        #[arg(long, default_value = "application/json")]
        content_type: String,
    },
    /// HTTP HEAD request
    Head {
        /// URL to check
        url: String,
    },
    /// Execute a full JSON HttpRequest
    Fetch {
        /// JSON HttpRequest object
        json: String,
    },
    /// Adaptive GET — tries HTTP first, escalates to CDP on anti-bot
    ///
    /// Uses Chrome-like TLS/headers, exponential backoff on 429,
    /// and automatic CDP escalation for Cloudflare challenges.
    Adaptive {
        /// URL to fetch
        url: String,
        /// Max retries before giving up (default: 3)
        #[arg(long, default_value = "3")]
        retries: u32,
        /// Disable CDP escalation (HTTP-only mode)
        #[arg(long)]
        no_escalate: bool,
        /// Custom User-Agent
        #[arg(long)]
        user_agent: Option<String>,
    },
}


#[derive(Subcommand)]
pub(crate) enum NetworkAction {
    /// Block resource types (comma-separated: image,stylesheet,font,script,media)
    Block {
        /// Resource types to block
        types: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum HarAction {
    /// Start HAR recording
    Start,
    /// Drain new HAR entries
    Drain,
    /// Export HAR 1.2 to file
    Export {
        /// Output file path
        #[arg(short, long, default_value = "recording.har")]
        output: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum WsAction {
    /// Start WebSocket interception
    Start,
    /// Drain captured frames
    Drain,
    /// Export frames to file
    Export {
        /// Output file path
        #[arg(short, long, default_value = "ws-frames.json")]
        output: String,
    },
    /// Show active WebSocket connections count
    Connections,
}


#[derive(Subcommand)]
pub(crate) enum ThrottleAction {
    /// Set a named throttling profile (fast3g, slow3g, offline, regular4g, wifi)
    Set {
        /// Profile name
        profile: String,
    },
    /// Set custom throttling conditions
    Custom {
        /// Download speed in kbps
        download_kbps: f64,
        /// Upload speed in kbps
        upload_kbps: f64,
        /// Latency in ms
        latency_ms: f64,
    },
    /// Clear network throttling
    Clear,
}


#[derive(Subcommand)]
pub(crate) enum NetworkLogAction {
    /// Start network request/response logging
    Start,
    /// Drain captured network entries (JSON)
    Drain,
    /// Get network summary statistics (JSON)
    Summary,
    /// Stop network logging
    Stop,
    /// Export network log to a JSON file
    Export {
        /// Output file path
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum ProxyAction {
    /// Create a proxy pool from JSON config
    CreatePool {
        /// JSON config for the proxy pool
        json: String,
    },
    /// Get Chrome launch args for a proxy pool
    ChromeArgs {
        /// Proxy pool JSON
        json: String,
    },
    /// Rotate to the next proxy in the pool
    Next {
        /// Proxy pool JSON
        json: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum ProxyHealthAction {
    /// Check a single proxy URL
    Check {
        /// Proxy URL (e.g. "http://proxy:8080")
        proxy: String,
        /// Custom test URL
        #[arg(long)]
        test_url: Option<String>,
        /// Timeout in ms
        #[arg(long, default_value = "10000")]
        timeout: u64,
    },
    /// Check multiple proxies from a JSON array
    CheckAll {
        /// JSON array of proxy URLs
        proxies_json: String,
    },
    /// Rank proxy health results by score (descending)
    Rank {
        /// JSON array of ProxyHealthResult
        results_json: String,
    },
    /// Filter proxy results by minimum score
    Filter {
        /// JSON array of ProxyHealthResult
        results_json: String,
        /// Minimum score threshold (0-100)
        min_score: u32,
    },
}


#[derive(Subcommand)]
pub(crate) enum InterceptCommandAction {
    /// Set interception rules (JSON array)
    Set {
        /// JSON array of InterceptRule
        rules_json: String,
    },
    /// Show intercepted request log
    Log,
    /// Clear all interception rules
    Clear,
}

