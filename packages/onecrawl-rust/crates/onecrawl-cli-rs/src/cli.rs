use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "onecrawl", version, about = "OneCrawl — AI-native browser automation", long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    // ── Session ──────────────────────────────────────────────────────
    /// Browser session management
    Session {
        #[command(subcommand)]
        action: crate::commands::session::SessionAction,
    },

    // ── Navigation ──────────────────────────────────────────────────
    /// Navigate to a URL
    Navigate {
        /// Target URL
        url: String,
        /// Wait after navigation (ms)
        #[arg(short, long, default_value = "0")]
        wait: u64,
        /// Auto-wait up to 30s for Cloudflare challenge to clear
        #[arg(long)]
        wait_cf: bool,
    },
    /// Go back in history
    Back,
    /// Go forward in history
    Forward,
    /// Reload the current page
    Reload,

    // ── Content ─────────────────────────────────────────────────────
    /// Get page content: text, html, url, title
    Get {
        /// What to get: text, html, url, title
        what: String,
        /// CSS selector (for text/html)
        selector: Option<String>,
    },
    /// Evaluate JavaScript expression
    Eval {
        /// JavaScript expression
        expression: String,
    },
    /// Set page HTML content
    SetContent {
        /// HTML content
        html: String,
    },

    // ── Element Interaction ─────────────────────────────────────────
    /// Click an element
    Click {
        /// CSS selector
        selector: String,
    },
    /// Double-click an element
    Dblclick {
        /// CSS selector
        selector: String,
    },
    /// Type text into an element (key-by-key)
    Type {
        /// CSS selector
        selector: String,
        /// Text to type
        text: String,
    },
    /// Fill an input field (clear + set value)
    Fill {
        /// CSS selector
        selector: String,
        /// Text to fill
        text: String,
    },
    /// Focus an element
    Focus {
        /// CSS selector
        selector: String,
    },
    /// Hover over an element
    Hover {
        /// CSS selector
        selector: String,
    },
    /// Scroll element into view
    ScrollIntoView {
        /// CSS selector
        selector: String,
    },
    /// Check a checkbox
    Check {
        /// CSS selector
        selector: String,
    },
    /// Uncheck a checkbox
    Uncheck {
        /// CSS selector
        selector: String,
    },
    /// Select an option in a <select> element
    SelectOption {
        /// CSS selector of the <select>
        selector: String,
        /// Option value to select
        value: String,
    },
    /// Tap an element (touch simulation)
    Tap {
        /// CSS selector
        selector: String,
    },
    /// Drag and drop between elements
    Drag {
        /// Source CSS selector
        from: String,
        /// Target CSS selector
        to: String,
    },
    /// Upload a file to a file input
    Upload {
        /// CSS selector of file input
        selector: String,
        /// Path to file
        file_path: String,
    },
    /// Get element bounding box (JSON)
    BoundingBox {
        /// CSS selector
        selector: String,
    },

    // ── Keyboard ────────────────────────────────────────────────────
    /// Press a key (keyDown + keyUp)
    PressKey {
        /// Key name (Enter, Tab, Escape, a-z, etc.)
        key: String,
    },
    /// Hold a key down
    KeyDown {
        /// Key name
        key: String,
    },
    /// Release a key
    KeyUp {
        /// Key name
        key: String,
    },
    /// Send a keyboard shortcut (e.g. "Control+a")
    KeyboardShortcut {
        /// Shortcut string (e.g. "Control+a", "Meta+c")
        keys: String,
    },

    // ── Screenshot / PDF ────────────────────────────────────────────
    /// Take a screenshot
    Screenshot {
        /// Output file path
        #[arg(short, long, default_value = "screenshot.png")]
        output: String,
        /// Full page screenshot
        #[arg(short, long)]
        full: bool,
        /// Screenshot a specific element
        #[arg(short, long)]
        element: Option<String>,
        /// Image format: png, jpeg, webp
        #[arg(long, default_value = "png")]
        format: String,
        /// JPEG/WebP quality (0-100)
        #[arg(short, long)]
        quality: Option<u32>,
    },
    /// Save page as PDF
    Pdf {
        /// Output file path
        #[arg(short, long, default_value = "page.pdf")]
        output: String,
        /// Landscape orientation
        #[arg(short, long)]
        landscape: bool,
        /// Page scale (default: 1.0)
        #[arg(short, long, default_value = "1.0")]
        scale: f64,
    },

    // ── Cookies ─────────────────────────────────────────────────────
    /// Cookie operations
    Cookie {
        #[command(subcommand)]
        action: CookieAction,
    },

    // ── Emulation ───────────────────────────────────────────────────
    /// Device and viewport emulation
    Emulate {
        #[command(subcommand)]
        action: EmulateAction,
    },

    // ── Network ─────────────────────────────────────────────────────
    /// Network operations
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },

    // ── HAR ─────────────────────────────────────────────────────────
    /// HAR recording
    Har {
        #[command(subcommand)]
        action: HarAction,
    },

    // ── WebSocket ───────────────────────────────────────────────────
    /// WebSocket interception
    Ws {
        #[command(subcommand)]
        action: WsAction,
    },

    // ── Coverage ────────────────────────────────────────────────────
    /// Code coverage
    Coverage {
        #[command(subcommand)]
        action: CoverageAction,
    },

    // ── Accessibility ───────────────────────────────────────────────
    /// Accessibility operations
    #[command(name = "a11y")]
    Accessibility {
        #[command(subcommand)]
        action: AccessibilityAction,
    },

    // ── Throttle ────────────────────────────────────────────────────
    /// Network throttling
    Throttle {
        #[command(subcommand)]
        action: ThrottleAction,
    },

    // ── Performance ─────────────────────────────────────────────────
    /// Performance tracing and metrics
    Perf {
        #[command(subcommand)]
        action: PerfAction,
    },

    // ── Console ─────────────────────────────────────────────────────
    /// Console message interception
    Console {
        #[command(subcommand)]
        action: ConsoleAction,
    },

    // ── Dialog ──────────────────────────────────────────────────────
    /// Dialog auto-handling (alert/confirm/prompt)
    Dialog {
        #[command(subcommand)]
        action: DialogAction,
    },

    // ── Worker ──────────────────────────────────────────────────────
    /// Service Worker management
    Worker {
        #[command(subcommand)]
        action: WorkerAction,
    },

    // ── DOM Observer ────────────────────────────────────────────────
    /// DOM mutation observation
    Dom {
        #[command(subcommand)]
        action: DomAction,
    },

    // ── Iframe ─────────────────────────────────────────────────────
    /// Iframe management
    Iframe {
        #[command(subcommand)]
        action: IframeAction,
    },

    // ── Network Log ────────────────────────────────────────────────
    /// Network request/response logging
    NetworkLog {
        #[command(subcommand)]
        action: NetworkLogAction,
    },

    // ── Page Watcher ───────────────────────────────────────────────
    /// Page state change watching
    PageWatcher {
        #[command(subcommand)]
        action: PageWatcherAction,
    },

    // ── Print (Enhanced) ───────────────────────────────────────────
    /// Enhanced PDF generation
    Print {
        #[command(subcommand)]
        action: PrintAction,
    },

    // ── Web Storage ─────────────────────────────────────────────────
    /// Web Storage operations (localStorage, sessionStorage, IndexedDB)
    WebStorage {
        #[command(subcommand)]
        action: WebStorageAction,
    },

    // ── Auth / Passkey ─────────────────────────────────────────────
    /// WebAuthn/passkey virtual authenticator
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    // ── Stealth ─────────────────────────────────────────────────────
    /// Stealth operations
    Stealth {
        #[command(subcommand)]
        action: StealthAction,
    },

    // ── Anti-Bot ────────────────────────────────────────────────────
    /// Advanced anti-bot evasion
    Antibot {
        #[command(subcommand)]
        action: AntibotAction,
    },

    // ── Adaptive Element Tracker ────────────────────────────────────
    /// Adaptive element fingerprinting and relocation
    Adaptive {
        #[command(subcommand)]
        action: AdaptiveAction,
    },

    // ── Wait ────────────────────────────────────────────────────────
    /// Wait for a duration in milliseconds
    Wait {
        /// Milliseconds to wait
        ms: u64,
    },
    /// Wait for a CSS selector to appear
    WaitForSelector {
        /// CSS selector
        selector: String,
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },
    /// Wait for URL to match a pattern
    WaitForUrl {
        /// URL substring to match
        url: String,
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },

    // ── Pages ───────────────────────────────────────────────────────
    /// Open a new browser page/tab
    NewPage {
        /// URL to open (default: about:blank)
        url: Option<String>,
    },

    // ── Proxy ───────────────────────────────────────────────────────
    /// Proxy pool management
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    // ── Proxy Health ────────────────────────────────────────────────
    /// Proxy health checking and scoring
    ProxyHealth {
        #[command(subcommand)]
        action: ProxyHealthAction,
    },

    // ── Request Interception ────────────────────────────────────────
    /// Request interception and mocking
    Intercept {
        #[command(subcommand)]
        action: InterceptCommandAction,
    },

    // ── Advanced Emulation ──────────────────────────────────────────
    /// Advanced emulation (sensors, permissions, hardware)
    AdvancedEmulation {
        #[command(subcommand)]
        action: AdvancedEmulationAction,
    },

    // ── Tab Management ──────────────────────────────────────────────
    /// Multi-tab management
    Tab {
        #[command(subcommand)]
        action: TabAction,
    },

    // ── Download Management ─────────────────────────────────────────
    /// File download management
    Download {
        #[command(subcommand)]
        action: DownloadAction,
    },

    // ── Screenshot Diff ─────────────────────────────────────────────
    /// Screenshot comparison and visual regression
    ScreenshotDiff {
        #[command(subcommand)]
        action: ScreenshotDiffAction,
    },

    // ── Geofencing ──────────────────────────────────────────────────
    /// Virtual geolocation profiles
    Geo {
        #[command(subcommand)]
        action: GeoAction,
    },

    // ── Cookie Jar ──────────────────────────────────────────────────
    /// Persistent cookie jar operations
    CookieJar {
        #[command(subcommand)]
        action: CookieJarAction,
    },

    // ── Request Queue ───────────────────────────────────────────────
    /// Queued request execution with retry
    Request {
        #[command(subcommand)]
        action: RequestAction,
    },

    // ── Offline Commands ────────────────────────────────────────────
    /// Crypto operations
    Crypto {
        #[command(subcommand)]
        action: crate::commands::crypto::CryptoAction,
    },
    /// Parse HTML
    Parse {
        #[command(subcommand)]
        action: crate::commands::parse::ParseAction,
    },
    /// Storage operations
    Storage {
        #[command(subcommand)]
        action: crate::commands::storage::StorageAction,
    },

    // ── System ──────────────────────────────────────────────────────
    /// Health check
    Health,
    /// Show version and system info
    Info,

    // ── Benchmark ────────────────────────────────────────────────────
    /// CDP benchmark suite
    Bench {
        #[command(subcommand)]
        action: BenchAction,
    },

    // ── Smart Selectors ─────────────────────────────────────────────
    /// CSS/XPath selectors with pseudo-elements (Scrapling-like)
    Select {
        #[command(subcommand)]
        action: SelectAction,
    },

    // ── DOM Navigation ──────────────────────────────────────────────
    /// DOM traversal — parent, siblings, children, above, below
    Nav {
        #[command(subcommand)]
        action: NavAction,
    },

    // ── Content Extraction ──────────────────────────────────────────
    /// Extract content as text, HTML, Markdown, or JSON
    Extract {
        #[command(subcommand)]
        action: ExtractAction,
    },

    // ── Spider / Crawl ─────────────────────────────────────────────
    /// Web spider/crawler
    Spider {
        #[command(subcommand)]
        action: SpiderAction,
    },

    // ── Robots.txt ────────────────────────────────────────────────
    /// Robots.txt parsing and compliance checking
    Robots {
        #[command(subcommand)]
        action: RobotsAction,
    },

    // ── Link Graph ────────────────────────────────────────────────
    /// Link graph analysis
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },

    // ── Interactive Shell ──────────────────────────────────────────
    /// Launch interactive scraping REPL
    Shell,

    // ── Domain Blocker ─────────────────────────────────────────────
    /// Block domains, ads, trackers and social scripts
    Domain {
        #[command(subcommand)]
        action: DomainAction,
    },

    // ── Streaming Extractor ────────────────────────────────────────
    /// Extract structured items from the page using CSS selectors
    StreamExtract {
        /// CSS selector for each item container
        item_selector: String,
        /// Field definitions: name:css:selector:extract (e.g. "title:css:h2:text")
        #[arg(short, long)]
        field: Vec<String>,
        /// Paginate: CSS selector for "next page" button
        #[arg(long)]
        paginate: Option<String>,
        /// Maximum pages to scrape (with --paginate)
        #[arg(long, default_value = "10")]
        max_pages: usize,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
        /// Output format: csv or json
        #[arg(long, default_value = "json")]
        format: String,
    },

    // ── TLS Fingerprint ──────────────────────────────────────────
    /// Browser TLS fingerprint management
    Fingerprint {
        #[command(subcommand)]
        action: FingerprintAction,
    },

    // ── Page Snapshot ────────────────────────────────────────────
    /// DOM snapshot and change detection
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },

    // ── HTTP Client ────────────────────────────────────────────────
    /// Execute HTTP requests via the browser's fetch API
    Http {
        #[command(subcommand)]
        action: HttpAction,
    },

    // ── Rate Limiter ──────────────────────────────────────────────
    /// Rate limiter for browser automation operations
    Ratelimit {
        #[command(subcommand)]
        action: RateLimitAction,
    },

    // ── Retry Queue ───────────────────────────────────────────────
    /// Retry queue for failed operations with exponential backoff
    Retry {
        #[command(subcommand)]
        action: RetryAction,
    },

    // ── Data Pipeline ─────────────────────────────────────────────
    /// Data processing pipeline for transforming and filtering scraped data
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },

    // ── Structured Data ───────────────────────────────────────────
    /// Extract structured data (JSON-LD, OpenGraph, Twitter Card, metadata)
    Structured {
        #[command(subcommand)]
        action: StructuredAction,
    },

    // ── Captcha ─────────────────────────────────────────────────────
    /// CAPTCHA detection and solution injection
    Captcha {
        #[command(subcommand)]
        action: CaptchaAction,
    },

    // ── Task Scheduler ──────────────────────────────────────────
    /// Task scheduler for browser automation
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },

    // ── Session Pool ────────────────────────────────────────────
    /// Session pool for parallel browser sessions
    Pool {
        #[command(subcommand)]
        action: PoolAction,
    },

    // ── Server ──────────────────────────────────────────────────
    /// Start the HTTP API server for multi-instance browser management
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 9867)]
        port: u16,
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
    },

    // ── MCP ─────────────────────────────────────────────────────
    /// Start the MCP (Model Context Protocol) server
    Mcp {
        /// Transport mode
        #[arg(short, long, default_value = "stdio")]
        transport: String,
    },

    // ── Version ─────────────────────────────────────────────────
    /// Show version and build information
    Version,
}

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
pub(crate) enum FingerprintAction {
    /// Apply a named fingerprint profile (chrome-win, chrome-mac, firefox-win, firefox-mac, safari-mac, edge-win)
    Apply {
        /// Profile name or "random"
        name: String,
    },
    /// Detect the current browser fingerprint
    Detect,
    /// List available fingerprint profiles
    List,
}

#[derive(Subcommand)]
pub(crate) enum SnapshotAction {
    /// Take a DOM snapshot of the current page
    Take {
        /// Output file path (JSON)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Compare two snapshot files
    Compare {
        /// First snapshot file
        path1: String,
        /// Second snapshot file
        path2: String,
    },
    /// Watch for DOM changes at regular intervals
    Watch {
        /// Interval in milliseconds between snapshots
        #[arg(short, long, default_value = "1000")]
        interval: u64,
        /// CSS selector to watch (optional)
        #[arg(short, long)]
        selector: Option<String>,
        /// Number of iterations (max 10)
        #[arg(short, long, default_value = "3")]
        count: usize,
    },
    /// Agent-mode snapshot: tag elements with @refs for AI-driven automation.
    ///
    /// Tags all visible interactive elements with data-onecrawl-ref attributes.
    /// After running, use @e1, @e2, ... in click/fill/get/hover commands.
    ///
    /// Example:
    ///   onecrawl snapshot agent --json
    ///   onecrawl click @e3
    ///   onecrawl fill @e5 "hello"
    Agent {
        /// Output machine-readable JSON: {"success":true,"data":{"snapshot":"...","refs":{...}}}
        #[arg(long)]
        json: bool,
        /// Only tag interactive elements (buttons, links, inputs). Default: false (includes headings/text).
        #[arg(long)]
        interactive_only: bool,
    },
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
}

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
pub(crate) enum PipelineAction {
    /// Run a pipeline on data
    Run {
        /// Path to pipeline definition JSON
        pipeline_json: String,
        /// Path to data JSON file (array of objects)
        data_json: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
        /// Output format: json, jsonl, csv
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Validate a pipeline definition
    Validate {
        /// Path to pipeline definition JSON
        pipeline_json: String,
    },
    /// Save a pipeline definition to a file
    Save {
        /// Pipeline definition JSON (inline)
        pipeline_json: String,
        /// Output file path
        path: String,
    },
    /// Load and display a pipeline from a file
    Load {
        /// Input file path
        path: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum StructuredAction {
    /// Extract all structured data from the current page
    ExtractAll,
    /// Extract JSON-LD from the current page
    JsonLd,
    /// Extract OpenGraph metadata from the current page
    OpenGraph,
    /// Extract Twitter Card metadata from the current page
    TwitterCard,
    /// Extract page metadata from the current page
    Metadata,
    /// Validate extracted structured data
    Validate {
        /// JSON string of StructuredDataResult
        data_json: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum CaptchaAction {
    /// Detect CAPTCHA presence on the current page
    Detect,
    /// Wait for a CAPTCHA to appear (with timeout)
    Wait {
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },
    /// Take a screenshot of the detected CAPTCHA element
    Screenshot,
    /// Inject a CAPTCHA solution token into the page
    Inject {
        /// Solution token
        solution: String,
    },
    /// List all detectable CAPTCHA types
    Types,
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
pub(crate) enum CookieJarAction {
    /// Export all cookies to stdout or file
    Export {
        /// Output file path (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import cookies from a file
    Import {
        /// Cookie jar JSON file path
        path: String,
    },
    /// Clear all cookies
    Clear,
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

#[derive(Subcommand)]
pub(crate) enum TabAction {
    /// List all open tabs
    List,
    /// Open a new tab
    New {
        /// URL to navigate to
        url: String,
    },
    /// Close a tab by index
    Close {
        /// Tab index (0-based)
        index: usize,
    },
    /// Switch to a tab by index
    Switch {
        /// Tab index (0-based)
        index: usize,
    },
    /// Get the count of open tabs
    Count,
}

#[derive(Subcommand)]
pub(crate) enum DownloadAction {
    /// Set download directory path
    SetPath {
        /// Directory path for downloads
        path: String,
    },
    /// List tracked downloads
    List,
    /// Download a file by URL (returns base64)
    Fetch {
        /// File URL
        url: String,
    },
    /// Wait for a download to appear
    Wait {
        /// Timeout in milliseconds
        #[arg(short, long, default_value = "10000")]
        timeout: u64,
    },
    /// Clear download history
    Clear,
}

#[derive(Subcommand)]
pub(crate) enum ScreenshotDiffAction {
    /// Compare two screenshot files
    Compare {
        /// Baseline screenshot path
        baseline: String,
        /// Current screenshot path
        current: String,
    },
    /// Visual regression against a baseline
    Regression {
        /// Baseline file path (created if missing)
        baseline_path: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum CookieAction {
    /// Get cookies
    Get {
        /// Filter by cookie name
        #[arg(short, long)]
        name: Option<String>,
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    /// Set a cookie
    Set {
        /// Cookie name
        name: String,
        /// Cookie value
        value: String,
        /// Cookie domain
        #[arg(short, long)]
        domain: Option<String>,
        /// Cookie path
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Delete a cookie
    Delete {
        /// Cookie name
        name: String,
        /// Cookie domain
        domain: String,
    },
    /// Clear all cookies
    Clear,
    /// Export all current page cookies to a JSON file (compatible with --import-cookies)
    Export {
        /// Output file path (defaults to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import cookies from a JSON file (format produced by 'cookie export')
    Import {
        /// Path to the JSON cookie file
        path: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum EmulateAction {
    /// Set viewport dimensions
    Viewport {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
        /// Device scale factor
        #[arg(short, long, default_value = "1.0")]
        scale: f64,
    },
    /// Emulate a known device
    Device {
        /// Device name: iphone_14, ipad, pixel_7, desktop
        name: String,
    },
    /// Override user agent
    UserAgent {
        /// User agent string
        ua: String,
    },
    /// Set geolocation
    Geolocation {
        /// Latitude
        lat: f64,
        /// Longitude
        lon: f64,
        /// Accuracy in meters
        #[arg(short, long, default_value = "1.0")]
        accuracy: f64,
    },
    /// Set color scheme preference
    ColorScheme {
        /// Scheme: dark or light
        scheme: String,
    },
    /// Clear all emulation overrides
    Clear,
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
pub(crate) enum CoverageAction {
    /// Start JS code coverage
    JsStart,
    /// Stop JS coverage and print report
    JsStop,
    /// Start CSS coverage
    CssStart,
    /// Get CSS coverage report
    CssReport,
}

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum AuthAction {
    /// Enable virtual WebAuthn authenticator
    PasskeyEnable {
        /// Protocol: ctap2 or u2f
        #[arg(long, default_value = "ctap2")]
        protocol: String,
        /// Transport: internal, usb, nfc, ble
        #[arg(long, default_value = "internal")]
        transport: String,
    },
    /// Add a passkey credential
    PasskeyAdd {
        /// Base64url-encoded credential ID
        #[arg(long)]
        credential_id: String,
        /// Relying party domain
        #[arg(long)]
        rp_id: String,
        /// Optional user handle
        #[arg(long)]
        user_handle: Option<String>,
    },
    /// List stored passkey credentials
    PasskeyList,
    /// Show passkey operation log
    PasskeyLog,
    /// Disable virtual authenticator
    PasskeyDisable,
    /// Remove a passkey credential
    PasskeyRemove {
        /// Credential ID to remove
        #[arg(long)]
        credential_id: String,
    },
    /// Enable CDP virtual authenticator, watch for passkey creation, export credential.
    ///
    /// Run this BEFORE registering a passkey on x.com (or other site):
    ///   1. Start a headed session and log in
    ///   2. Run `auth passkey-register --output /tmp/xcom-passkey.json`
    ///   3. Register the passkey in the browser (x.com Settings → Security → Passkey)
    ///   4. The credential is exported automatically when Chrome records it
    PasskeyRegister {
        /// File to write the exported passkey credentials (JSON)
        #[arg(long, default_value = "/tmp/onecrawl-passkeys.json")]
        output: String,
        /// Seconds to wait for the passkey to be registered (default: 120)
        #[arg(long, default_value_t = 120u64)]
        timeout_secs: u64,
    },
    /// Store a passkey file path in the session so CDP WebAuthn is re-injected
    /// on every connect. Use with `session start --import-passkey FILE`.
    PasskeySetFile {
        /// Path to passkey JSON file produced by `auth passkey-register`
        #[arg(long)]
        file: String,
    },

    // ── Vault commands ───────────────────────────────────────────────

    /// List all sites and credentials stored in the passkey vault (~/.onecrawl/passkeys/vault.json).
    VaultList,

    /// Save credentials from a native passkey JSON file into the vault.
    VaultSave {
        /// Path to passkey JSON file (produced by `auth passkey-register`)
        #[arg(long)]
        input: String,
    },

    /// Remove a credential from the vault by its credential_id.
    VaultRemove {
        /// Base64-encoded credential ID to remove
        #[arg(long)]
        credential_id: String,
    },

    /// Remove all credentials for a specific rp_id (site) from the vault.
    VaultClearSite {
        /// Relying party ID, e.g. `x.com`
        #[arg(long)]
        rp_id: String,
    },

    /// Export vault credentials for a site to a passkey JSON file.
    VaultExport {
        /// Relying party ID, e.g. `x.com`
        #[arg(long)]
        rp_id: String,
        /// Output file path
        #[arg(long, default_value = "/tmp/onecrawl-passkeys.json")]
        output: String,
    },

    /// Import passkeys from a Bitwarden unencrypted JSON export.
    ///
    /// Generate with: `bw export --format json --output export.json`
    /// Note: Only Bitwarden-native passkeys are importable. Apple/Windows
    /// hardware-bound passkeys cannot be exported by design.
    ImportBitwarden {
        /// Path to Bitwarden JSON export file
        #[arg(long)]
        input: String,
        /// Also save imported credentials to the vault
        #[arg(long, default_value_t = true)]
        vault: bool,
    },

    /// Import passkeys from a 1Password export (export.data JSON from a .1pux archive).
    ///
    /// Extract the .1pux ZIP first: `unzip export.1pux export.data`
    ImportOnePassword {
        /// Path to `export.data` JSON file (extracted from .1pux)
        #[arg(long)]
        input: String,
        /// Also save imported credentials to the vault
        #[arg(long, default_value_t = true)]
        vault: bool,
    },

    /// Import passkeys from a FIDO Alliance CXF (Credential Exchange Format) JSON file.
    ///
    /// CXF v1.0 (FIDO Alliance draft, Oct 2024). For encrypted CXF files,
    /// decrypt first — HPKE-encrypted CXF is not yet supported.
    ImportCxf {
        /// Path to CXF JSON file (`cxf.json`)
        #[arg(long)]
        input: String,
        /// Also save imported credentials to the vault
        #[arg(long, default_value_t = true)]
        vault: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum StealthAction {
    /// Inject stealth anti-detection patches
    Inject,
}

#[derive(Subcommand)]
pub(crate) enum AntibotAction {
    /// Inject full anti-bot stealth patches
    Inject {
        /// Level: basic, standard, aggressive
        #[arg(short, long, default_value = "aggressive")]
        level: String,
    },
    /// Run bot detection test on the current page
    Test,
    /// List available stealth profiles
    Profiles,
}

#[derive(Subcommand)]
pub(crate) enum AdaptiveAction {
    /// Fingerprint a DOM element by CSS selector
    Fingerprint {
        /// CSS selector
        selector: String,
    },
    /// Relocate an element using a fingerprint JSON
    Relocate {
        /// Fingerprint JSON string
        fingerprint_json: String,
    },
    /// Track multiple elements by selectors (JSON array)
    Track {
        /// JSON array of CSS selectors
        selectors: String,
        /// Optional path to save fingerprints
        #[arg(short, long)]
        save: Option<String>,
    },
    /// Relocate all tracked elements from fingerprints JSON
    RelocateAll {
        /// JSON array of fingerprints
        fingerprints_json: String,
    },
    /// Save fingerprints JSON to a file
    Save {
        /// JSON array of fingerprints
        fingerprints: String,
        /// File path
        path: String,
    },
    /// Load fingerprints from a file
    Load {
        /// File path
        path: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum AccessibilityAction {
    /// Get the full accessibility tree
    Tree,
    /// Get accessibility info for an element
    Element {
        /// CSS selector
        selector: String,
    },
    /// Run an accessibility audit
    Audit,
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
pub(crate) enum PerfAction {
    /// Start performance tracing
    TraceStart,
    /// Stop tracing and print trace data
    TraceStop,
    /// Get performance metrics
    Metrics,
    /// Get navigation timing
    Timing,
    /// Get resource timing entries
    Resources,
}

#[derive(Subcommand)]
pub(crate) enum ConsoleAction {
    /// Start console message capture
    Start,
    /// Drain captured console entries (JSON)
    Drain,
    /// Clear the console buffer
    Clear,
}

#[derive(Subcommand)]
pub(crate) enum DialogAction {
    /// Set dialog auto-handler
    SetHandler {
        /// Accept dialogs
        #[arg(long)]
        accept: bool,
        /// Text to return for prompt() dialogs
        #[arg(long)]
        prompt_text: Option<String>,
    },
    /// Get dialog history (JSON)
    History,
    /// Clear dialog history
    Clear,
}

#[derive(Subcommand)]
pub(crate) enum WorkerAction {
    /// List registered service workers
    List,
    /// Unregister all service workers
    Unregister,
    /// Get detailed worker info (JSON)
    Info,
}

#[derive(Subcommand)]
pub(crate) enum DomAction {
    /// Start observing DOM mutations
    Observe {
        /// CSS selector for the target element
        #[arg(short, long)]
        selector: Option<String>,
    },
    /// Drain accumulated DOM mutations (JSON)
    Mutations,
    /// Stop the DOM observer
    Stop,
    /// Get a snapshot of the current DOM as HTML
    Snapshot {
        /// CSS selector to snapshot (default: full document)
        #[arg(short, long)]
        selector: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum IframeAction {
    /// List all iframes on the page (JSON)
    List,
    /// Execute JavaScript inside an iframe
    Eval {
        /// Iframe index (0-based)
        index: usize,
        /// JavaScript expression to evaluate
        expression: String,
    },
    /// Get the HTML content of an iframe
    Content {
        /// Iframe index (0-based)
        index: usize,
    },
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
pub(crate) enum PageWatcherAction {
    /// Start watching for page state changes
    Start,
    /// Drain accumulated page changes (JSON)
    Drain,
    /// Stop the page watcher
    Stop,
    /// Get current page state snapshot (JSON)
    State,
}

#[derive(Subcommand)]
pub(crate) enum PrintAction {
    /// Generate PDF with detailed options
    Pdf {
        /// Output file path
        #[arg(short, long, default_value = "output.pdf")]
        output: String,
        /// Landscape orientation
        #[arg(long)]
        landscape: bool,
        /// Print background graphics
        #[arg(long)]
        background: bool,
        /// Page scale factor
        #[arg(long)]
        scale: Option<f64>,
        /// Paper width in inches
        #[arg(long)]
        paper_width: Option<f64>,
        /// Paper height in inches
        #[arg(long)]
        paper_height: Option<f64>,
        /// Margins as "top,bottom,left,right" in inches
        #[arg(long)]
        margins: Option<String>,
        /// Page ranges (e.g. "1-3,5")
        #[arg(long)]
        page_ranges: Option<String>,
        /// Header HTML template
        #[arg(long)]
        header: Option<String>,
        /// Footer HTML template
        #[arg(long)]
        footer: Option<String>,
    },
    /// Get page print preview metrics (JSON)
    Metrics,
}

#[derive(Subcommand)]
pub(crate) enum WebStorageAction {
    /// Get all localStorage contents (JSON)
    LocalGet,
    /// Set a localStorage item
    LocalSet {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Clear all localStorage
    LocalClear,
    /// Get all sessionStorage contents (JSON)
    SessionGet,
    /// Set a sessionStorage item
    SessionSet {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Clear all sessionStorage
    SessionClear,
    /// List IndexedDB database names
    IndexeddbList,
    /// Clear all site data (localStorage + sessionStorage + cookies + cache)
    ClearAll,
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

#[derive(Subcommand)]
pub(crate) enum AdvancedEmulationAction {
    /// Override device orientation sensor
    Orientation {
        /// Rotation around z-axis
        alpha: f64,
        /// Rotation around x-axis
        beta: f64,
        /// Rotation around y-axis
        gamma: f64,
    },
    /// Override a permission query result
    Permission {
        /// Permission name (e.g. geolocation, camera, microphone)
        name: String,
        /// State: granted, denied, prompt
        state: String,
    },
    /// Override battery status
    Battery {
        /// Battery level (0.0–1.0)
        level: f64,
        /// Whether the device is charging
        #[arg(long)]
        charging: bool,
    },
    /// Override Network Information API
    Connection {
        /// Effective type (e.g. 4g, 3g, 2g, slow-2g)
        effective_type: String,
        /// Downlink speed in Mbps
        downlink: f64,
        /// Round-trip time in ms
        rtt: u32,
    },
    /// Override CPU core count
    CpuCores {
        /// Number of CPU cores
        n: u32,
    },
    /// Override device memory
    Memory {
        /// Device memory in GB
        gb: f64,
    },
    /// Get current navigator properties
    NavigatorInfo,
}

#[derive(Subcommand)]
pub(crate) enum SelectAction {
    /// CSS selector (supports ::text, ::attr(name) pseudo-elements)
    Css {
        /// CSS selector string
        selector: String,
    },
    /// XPath selector
    Xpath {
        /// XPath expression
        expression: String,
    },
    /// Find elements by text content
    Text {
        /// Text to search for
        text: String,
        /// Filter by tag name
        #[arg(long)]
        tag: Option<String>,
    },
    /// Find elements by regex pattern
    Regex {
        /// Regex pattern
        pattern: String,
        /// Filter by tag name
        #[arg(long)]
        tag: Option<String>,
    },
    /// Auto-generate a unique CSS selector for an element
    AutoSelector {
        /// Target CSS selector
        selector: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum NavAction {
    /// Get parent element
    Parent {
        /// CSS selector
        selector: String,
    },
    /// Get child elements
    Children {
        /// CSS selector
        selector: String,
    },
    /// Get next sibling element
    NextSibling {
        /// CSS selector
        selector: String,
    },
    /// Get previous sibling element
    PrevSibling {
        /// CSS selector
        selector: String,
    },
    /// Get all sibling elements
    Siblings {
        /// CSS selector
        selector: String,
    },
    /// Find similar elements
    Similar {
        /// CSS selector
        selector: String,
    },
    /// Get elements above the target
    Above {
        /// CSS selector
        selector: String,
        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Get elements below the target
    Below {
        /// CSS selector
        selector: String,
        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub(crate) enum ExtractAction {
    /// Extract content in a given format (text, html, markdown, json)
    Content {
        /// Output format: text, html, markdown, json
        format: String,
        /// CSS selector to scope extraction
        #[arg(long)]
        selector: Option<String>,
        /// Save output to file
        #[arg(long)]
        output: Option<String>,
    },
    /// Get structured page metadata
    Metadata,
}

#[derive(Subcommand)]
pub(crate) enum SpiderAction {
    /// Crawl starting from a URL
    Crawl {
        /// Start URL
        start_url: String,
        /// Maximum crawl depth
        #[arg(long, default_value = "3")]
        max_depth: usize,
        /// Maximum number of pages
        #[arg(long, default_value = "100")]
        max_pages: usize,
        /// Concurrent workers (reserved for future use)
        #[arg(long, default_value = "3")]
        concurrency: usize,
        /// Delay between requests in milliseconds
        #[arg(long, default_value = "500")]
        delay: u64,
        /// Only follow links on the same domain
        #[arg(long, default_value = "true")]
        same_domain: bool,
        /// CSS selector to extract from each page
        #[arg(long)]
        selector: Option<String>,
        /// Content format: text, html, markdown, json
        #[arg(long, default_value = "text")]
        format: String,
        /// Save results to file
        #[arg(long)]
        output: Option<String>,
        /// Output file format: json or jsonl
        #[arg(long, default_value = "json")]
        output_format: String,
    },
    /// Resume a crawl from a saved state file
    Resume {
        /// Path to the state JSON file
        state_file: String,
    },
    /// Print summary of a results file
    Summary {
        /// Path to the results JSON file
        results_file: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum RobotsAction {
    /// Parse robots.txt from a URL or local file
    Parse {
        /// URL or file path to robots.txt
        source: String,
    },
    /// Check if a path is allowed by robots.txt
    Check {
        /// URL to the site (fetches /robots.txt)
        url: String,
        /// Path to check
        path: String,
        /// User-agent string
        #[arg(long, default_value = "*")]
        user_agent: String,
    },
    /// List sitemaps declared in robots.txt
    Sitemaps {
        /// URL to the site (fetches /robots.txt)
        url: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum GraphAction {
    /// Extract links from the current page
    Extract {
        /// Base URL for internal/external classification
        #[arg(long)]
        base_url: Option<String>,
    },
    /// Build a graph from edges JSON file
    Build {
        /// Path to edges JSON file
        edges_json: String,
    },
    /// Analyze a graph JSON file
    Analyze {
        /// Path to graph JSON file
        graph_json: String,
    },
    /// Export graph to a JSON file
    Export {
        /// Path to graph JSON file
        graph_json: String,
        /// Output file path
        output_path: String,
    },
}
