use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "onecrawl", version, about = "OneCrawl — AI-native browser automation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // ── Session ──────────────────────────────────────────────────────
    /// Browser session management
    Session {
        #[command(subcommand)]
        action: commands::session::SessionAction,
    },

    // ── Navigation ──────────────────────────────────────────────────
    /// Navigate to a URL
    Navigate {
        /// Target URL
        url: String,
        /// Wait after navigation (ms)
        #[arg(short, long, default_value = "0")]
        wait: u64,
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

    // ── Stealth ─────────────────────────────────────────────────────
    /// Stealth operations
    Stealth {
        #[command(subcommand)]
        action: StealthAction,
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
        action: commands::crypto::CryptoAction,
    },
    /// Parse HTML
    Parse {
        #[command(subcommand)]
        action: commands::parse::ParseAction,
    },
    /// Storage operations
    Storage {
        #[command(subcommand)]
        action: commands::storage::StorageAction,
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
}

#[derive(Subcommand)]
enum BenchAction {
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
enum GeoAction {
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
enum CookieJarAction {
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
enum RequestAction {
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
enum TabAction {
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
enum DownloadAction {
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
enum ScreenshotDiffAction {
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
enum CookieAction {
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
}

#[derive(Subcommand)]
enum EmulateAction {
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
enum NetworkAction {
    /// Block resource types (comma-separated: image,stylesheet,font,script,media)
    Block {
        /// Resource types to block
        types: String,
    },
}

#[derive(Subcommand)]
enum HarAction {
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
enum WsAction {
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
enum CoverageAction {
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
enum StealthAction {
    /// Inject stealth anti-detection patches
    Inject,
}

#[derive(Subcommand)]
enum AccessibilityAction {
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
enum ThrottleAction {
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
enum PerfAction {
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
enum ConsoleAction {
    /// Start console message capture
    Start,
    /// Drain captured console entries (JSON)
    Drain,
    /// Clear the console buffer
    Clear,
}

#[derive(Subcommand)]
enum DialogAction {
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
enum WorkerAction {
    /// List registered service workers
    List,
    /// Unregister all service workers
    Unregister,
    /// Get detailed worker info (JSON)
    Info,
}

#[derive(Subcommand)]
enum DomAction {
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
enum IframeAction {
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
enum NetworkLogAction {
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
enum PageWatcherAction {
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
enum PrintAction {
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
enum WebStorageAction {
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
enum ProxyAction {
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
enum InterceptCommandAction {
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
enum AdvancedEmulationAction {
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
enum SelectAction {
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
enum NavAction {
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
enum ExtractAction {
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── System ──────────────────────────────────────────────────
        Commands::Health => {
            println!("✅ OneCrawl Rust CLI v{}", env!("CARGO_PKG_VERSION"));
            println!("   Crates: core, crypto, parser, storage, cdp");
            println!("   Runtime: Tokio async");
        }
        Commands::Info => {
            println!("OneCrawl v{}", env!("CARGO_PKG_VERSION"));
            println!("Arch: {}", std::env::consts::ARCH);
            println!("OS: {}", std::env::consts::OS);
            println!("Rust: compiled native binary");
        }

        // ── Offline Commands (untouched) ────────────────────────────
        Commands::Crypto { action } => commands::crypto::handle(action),
        Commands::Parse { action } => commands::parse::handle(action),
        Commands::Storage { action } => commands::storage::handle(action).await,

        // ── Session ─────────────────────────────────────────────────
        Commands::Session { action } => commands::session::handle(action).await,

        // ── Navigation ──────────────────────────────────────────────
        Commands::Navigate { url, wait } => commands::browser::navigate(&url, wait).await,
        Commands::Back => commands::browser::back().await,
        Commands::Forward => commands::browser::forward().await,
        Commands::Reload => commands::browser::reload().await,

        // ── Content ─────────────────────────────────────────────────
        Commands::Get { what, selector } => {
            commands::browser::get(&what, selector.as_deref()).await
        }
        Commands::Eval { expression } => commands::browser::eval(&expression).await,
        Commands::SetContent { html } => commands::browser::set_content(&html).await,

        // ── Element Interaction ─────────────────────────────────────
        Commands::Click { selector } => commands::browser::click(&selector).await,
        Commands::Dblclick { selector } => commands::browser::dblclick(&selector).await,
        Commands::Type { selector, text } => commands::browser::type_text(&selector, &text).await,
        Commands::Fill { selector, text } => commands::browser::fill(&selector, &text).await,
        Commands::Focus { selector } => commands::browser::focus(&selector).await,
        Commands::Hover { selector } => commands::browser::hover(&selector).await,
        Commands::ScrollIntoView { selector } => {
            commands::browser::scroll_into_view(&selector).await
        }
        Commands::Check { selector } => commands::browser::check(&selector).await,
        Commands::Uncheck { selector } => commands::browser::uncheck(&selector).await,
        Commands::SelectOption { selector, value } => {
            commands::browser::select_option(&selector, &value).await
        }
        Commands::Tap { selector } => commands::browser::tap(&selector).await,
        Commands::Drag { from, to } => commands::browser::drag(&from, &to).await,
        Commands::Upload {
            selector,
            file_path,
        } => commands::browser::upload(&selector, &file_path).await,
        Commands::BoundingBox { selector } => commands::browser::bounding_box(&selector).await,

        // ── Keyboard ────────────────────────────────────────────────
        Commands::PressKey { key } => commands::browser::press_key(&key).await,
        Commands::KeyDown { key } => commands::browser::key_down(&key).await,
        Commands::KeyUp { key } => commands::browser::key_up(&key).await,
        Commands::KeyboardShortcut { keys } => commands::browser::keyboard_shortcut(&keys).await,

        // ── Screenshot / PDF ────────────────────────────────────────
        Commands::Screenshot {
            output,
            full,
            element,
            format,
            quality,
        } => {
            commands::browser::screenshot(&output, full, element.as_deref(), &format, quality).await
        }
        Commands::Pdf {
            output,
            landscape,
            scale,
        } => commands::browser::pdf(&output, landscape, scale).await,

        // ── Cookies ─────────────────────────────────────────────────
        Commands::Cookie { action } => match action {
            CookieAction::Get { name, json } => {
                commands::browser::cookie_get(name.as_deref(), json).await
            }
            CookieAction::Set {
                name,
                value,
                domain,
                path,
            } => {
                commands::browser::cookie_set(&name, &value, domain.as_deref(), path.as_deref())
                    .await
            }
            CookieAction::Delete { name, domain } => {
                commands::browser::cookie_delete(&name, &domain).await
            }
            CookieAction::Clear => commands::browser::cookie_clear().await,
        },

        // ── Emulation ───────────────────────────────────────────────
        Commands::Emulate { action } => match action {
            EmulateAction::Viewport {
                width,
                height,
                scale,
            } => commands::browser::emulate_viewport(width, height, scale).await,
            EmulateAction::Device { name } => commands::browser::emulate_device(&name).await,
            EmulateAction::UserAgent { ua } => commands::browser::emulate_user_agent(&ua).await,
            EmulateAction::Geolocation { lat, lon, accuracy } => {
                commands::browser::emulate_geolocation(lat, lon, accuracy).await
            }
            EmulateAction::ColorScheme { scheme } => {
                commands::browser::emulate_color_scheme(&scheme).await
            }
            EmulateAction::Clear => commands::browser::emulate_clear().await,
        },

        // ── Network ─────────────────────────────────────────────────
        Commands::Network { action } => match action {
            NetworkAction::Block { types } => commands::browser::network_block(&types).await,
        },

        // ── HAR ─────────────────────────────────────────────────────
        Commands::Har { action } => match action {
            HarAction::Start => commands::browser::har_start().await,
            HarAction::Drain => commands::browser::har_drain().await,
            HarAction::Export { output } => commands::browser::har_export(&output).await,
        },

        // ── WebSocket ───────────────────────────────────────────────
        Commands::Ws { action } => match action {
            WsAction::Start => commands::browser::ws_start().await,
            WsAction::Drain => commands::browser::ws_drain().await,
            WsAction::Export { output } => commands::browser::ws_export(&output).await,
            WsAction::Connections => commands::browser::ws_connections().await,
        },

        // ── Coverage ────────────────────────────────────────────────
        Commands::Coverage { action } => match action {
            CoverageAction::JsStart => commands::browser::coverage_js_start().await,
            CoverageAction::JsStop => commands::browser::coverage_js_stop().await,
            CoverageAction::CssStart => commands::browser::coverage_css_start().await,
            CoverageAction::CssReport => commands::browser::coverage_css_report().await,
        },

        // ── Accessibility ───────────────────────────────────────────
        Commands::Accessibility { action } => match action {
            AccessibilityAction::Tree => commands::browser::a11y_tree().await,
            AccessibilityAction::Element { selector } => {
                commands::browser::a11y_element(&selector).await
            }
            AccessibilityAction::Audit => commands::browser::a11y_audit().await,
        },

        // ── Throttle ────────────────────────────────────────────────
        Commands::Throttle { action } => match action {
            ThrottleAction::Set { profile } => commands::browser::throttle_set(&profile).await,
            ThrottleAction::Custom {
                download_kbps,
                upload_kbps,
                latency_ms,
            } => commands::browser::throttle_custom(download_kbps, upload_kbps, latency_ms).await,
            ThrottleAction::Clear => commands::browser::throttle_clear().await,
        },

        // ── Performance ─────────────────────────────────────────────
        Commands::Perf { action } => match action {
            PerfAction::TraceStart => commands::browser::perf_trace_start().await,
            PerfAction::TraceStop => commands::browser::perf_trace_stop().await,
            PerfAction::Metrics => commands::browser::perf_metrics().await,
            PerfAction::Timing => commands::browser::perf_timing().await,
            PerfAction::Resources => commands::browser::perf_resources().await,
        },

        // ── Console ─────────────────────────────────────────────────
        Commands::Console { action } => match action {
            ConsoleAction::Start => commands::browser::console_start().await,
            ConsoleAction::Drain => commands::browser::console_drain().await,
            ConsoleAction::Clear => commands::browser::console_clear().await,
        },

        // ── Dialog ──────────────────────────────────────────────────
        Commands::Dialog { action } => match action {
            DialogAction::SetHandler {
                accept,
                prompt_text,
            } => commands::browser::dialog_set_handler(accept, prompt_text.as_deref()).await,
            DialogAction::History => commands::browser::dialog_history().await,
            DialogAction::Clear => commands::browser::dialog_clear().await,
        },

        // ── Worker ──────────────────────────────────────────────────
        Commands::Worker { action } => match action {
            WorkerAction::List => commands::browser::worker_list().await,
            WorkerAction::Unregister => commands::browser::worker_unregister().await,
            WorkerAction::Info => commands::browser::worker_info().await,
        },

        // ── DOM Observer ────────────────────────────────────────────
        Commands::Dom { action } => match action {
            DomAction::Observe { selector } => {
                commands::browser::dom_observe(selector.as_deref()).await
            }
            DomAction::Mutations => commands::browser::dom_mutations().await,
            DomAction::Stop => commands::browser::dom_stop().await,
            DomAction::Snapshot { selector } => {
                commands::browser::dom_snapshot(selector.as_deref()).await
            }
        },

        // ── Iframe ──────────────────────────────────────────────────
        Commands::Iframe { action } => match action {
            IframeAction::List => commands::browser::iframe_list().await,
            IframeAction::Eval { index, expression } => {
                commands::browser::iframe_eval(index, &expression).await
            }
            IframeAction::Content { index } => commands::browser::iframe_content(index).await,
        },

        // ── Network Log ─────────────────────────────────────────────
        Commands::NetworkLog { action } => match action {
            NetworkLogAction::Start => commands::browser::network_log_start().await,
            NetworkLogAction::Drain => commands::browser::network_log_drain().await,
            NetworkLogAction::Summary => commands::browser::network_log_summary().await,
            NetworkLogAction::Stop => commands::browser::network_log_stop().await,
            NetworkLogAction::Export { path } => {
                commands::browser::network_log_export(&path).await
            }
        },

        // ── Page Watcher ────────────────────────────────────────────
        Commands::PageWatcher { action } => match action {
            PageWatcherAction::Start => commands::browser::page_watcher_start().await,
            PageWatcherAction::Drain => commands::browser::page_watcher_drain().await,
            PageWatcherAction::Stop => commands::browser::page_watcher_stop().await,
            PageWatcherAction::State => commands::browser::page_watcher_state().await,
        },

        // ── Print (Enhanced) ────────────────────────────────────────
        Commands::Print { action } => match action {
            PrintAction::Pdf {
                output,
                landscape,
                background,
                scale,
                paper_width,
                paper_height,
                margins,
                page_ranges,
                header,
                footer,
            } => {
                commands::browser::print_pdf(
                    &output,
                    landscape,
                    background,
                    scale,
                    paper_width,
                    paper_height,
                    margins.as_deref(),
                    page_ranges,
                    header,
                    footer,
                )
                .await
            }
            PrintAction::Metrics => commands::browser::print_metrics().await,
        },

        // ── Web Storage ─────────────────────────────────────────────
        Commands::WebStorage { action } => match action {
            WebStorageAction::LocalGet => commands::browser::web_storage_local_get().await,
            WebStorageAction::LocalSet { key, value } => {
                commands::browser::web_storage_local_set(&key, &value).await
            }
            WebStorageAction::LocalClear => commands::browser::web_storage_local_clear().await,
            WebStorageAction::SessionGet => commands::browser::web_storage_session_get().await,
            WebStorageAction::SessionSet { key, value } => {
                commands::browser::web_storage_session_set(&key, &value).await
            }
            WebStorageAction::SessionClear => commands::browser::web_storage_session_clear().await,
            WebStorageAction::IndexeddbList => {
                commands::browser::web_storage_indexeddb_list().await
            }
            WebStorageAction::ClearAll => commands::browser::web_storage_clear_all().await,
        },

        // ── Stealth ─────────────────────────────────────────────────
        Commands::Stealth { action } => match action {
            StealthAction::Inject => commands::browser::stealth_inject().await,
        },

        // ── Wait ────────────────────────────────────────────────────
        Commands::Wait { ms } => commands::browser::wait_ms(ms).await,
        Commands::WaitForSelector { selector, timeout } => {
            commands::browser::wait_for_selector(&selector, timeout).await
        }
        Commands::WaitForUrl { url, timeout } => {
            commands::browser::wait_for_url(&url, timeout).await
        }

        // ── Pages ───────────────────────────────────────────────────
        Commands::NewPage { url } => commands::browser::new_page(url.as_deref()).await,

        // ── Proxy ───────────────────────────────────────────────────
        Commands::Proxy { action } => match action {
            ProxyAction::CreatePool { json } => commands::browser::proxy_create_pool(&json).await,
            ProxyAction::ChromeArgs { json } => commands::browser::proxy_chrome_args(&json).await,
            ProxyAction::Next { json } => commands::browser::proxy_next(&json).await,
        },

        // ── Request Interception ────────────────────────────────────
        Commands::Intercept { action } => match action {
            InterceptCommandAction::Set { rules_json } => {
                commands::browser::intercept_set(&rules_json).await
            }
            InterceptCommandAction::Log => commands::browser::intercept_log().await,
            InterceptCommandAction::Clear => commands::browser::intercept_clear().await,
        },

        // ── Advanced Emulation ──────────────────────────────────────
        Commands::AdvancedEmulation { action } => match action {
            AdvancedEmulationAction::Orientation { alpha, beta, gamma } => {
                commands::browser::adv_emulation_orientation(alpha, beta, gamma).await
            }
            AdvancedEmulationAction::Permission { name, state } => {
                commands::browser::adv_emulation_permission(&name, &state).await
            }
            AdvancedEmulationAction::Battery { level, charging } => {
                commands::browser::adv_emulation_battery(level, charging).await
            }
            AdvancedEmulationAction::Connection {
                effective_type,
                downlink,
                rtt,
            } => commands::browser::adv_emulation_connection(&effective_type, downlink, rtt).await,
            AdvancedEmulationAction::CpuCores { n } => {
                commands::browser::adv_emulation_cpu_cores(n).await
            }
            AdvancedEmulationAction::Memory { gb } => {
                commands::browser::adv_emulation_memory(gb).await
            }
            AdvancedEmulationAction::NavigatorInfo => {
                commands::browser::adv_emulation_navigator_info().await
            }
        },

        // ── Tab Management ──────────────────────────────────────────
        Commands::Tab { action } => match action {
            TabAction::List => commands::browser::tab_list().await,
            TabAction::New { url } => commands::browser::tab_new(&url).await,
            TabAction::Close { index } => commands::browser::tab_close(index).await,
            TabAction::Switch { index } => commands::browser::tab_switch(index).await,
            TabAction::Count => commands::browser::tab_count_cmd().await,
        },

        // ── Download Management ─────────────────────────────────────
        Commands::Download { action } => match action {
            DownloadAction::SetPath { path } => commands::browser::download_set_path(&path).await,
            DownloadAction::List => commands::browser::download_list().await,
            DownloadAction::Fetch { url } => commands::browser::download_fetch(&url).await,
            DownloadAction::Wait { timeout } => commands::browser::download_wait(timeout).await,
            DownloadAction::Clear => commands::browser::download_clear().await,
        },

        // ── Screenshot Diff ─────────────────────────────────────────
        Commands::ScreenshotDiff { action } => match action {
            ScreenshotDiffAction::Compare { baseline, current } => {
                commands::browser::screenshot_diff_compare(&baseline, &current).await
            }
            ScreenshotDiffAction::Regression { baseline_path } => {
                commands::browser::screenshot_diff_regression(&baseline_path).await
            }
        },

        // ── Geofencing ─────────────────────────────────────────────
        Commands::Geo { action } => match action {
            GeoAction::Apply { profile } => commands::browser::geo_apply(&profile).await,
            GeoAction::Presets => commands::browser::geo_presets().await,
            GeoAction::Current => commands::browser::geo_current().await,
        },

        // ── Cookie Jar ─────────────────────────────────────────────
        Commands::CookieJar { action } => match action {
            CookieJarAction::Export { output } => {
                commands::browser::cookie_jar_export(output.as_deref()).await
            }
            CookieJarAction::Import { path } => commands::browser::cookie_jar_import(&path).await,
            CookieJarAction::Clear => commands::browser::cookie_jar_clear().await,
        },

        // ── Request Queue ──────────────────────────────────────────
        Commands::Request { action } => match action {
            RequestAction::Execute { json } => commands::browser::request_execute(&json).await,
            RequestAction::Batch {
                json,
                concurrency,
                delay,
            } => commands::browser::request_batch(&json, concurrency, delay).await,
        },

        // ── Benchmark ───────────────────────────────────────────────
        Commands::Bench { action } => match action {
            BenchAction::Run { iterations, module } => {
                commands::browser::bench_run(iterations, module.as_deref()).await
            }
            BenchAction::Report { format } => commands::browser::bench_report(&format).await,
        },

        // ── Smart Selectors ─────────────────────────────────────────
        Commands::Select { action } => match action {
            SelectAction::Css { selector } => commands::browser::select_css(&selector).await,
            SelectAction::Xpath { expression } => {
                commands::browser::select_xpath(&expression).await
            }
            SelectAction::Text { text, tag } => {
                commands::browser::select_text(&text, tag.as_deref()).await
            }
            SelectAction::Regex { pattern, tag } => {
                commands::browser::select_regex(&pattern, tag.as_deref()).await
            }
            SelectAction::AutoSelector { selector } => {
                commands::browser::select_auto(&selector).await
            }
        },

        // ── DOM Navigation ──────────────────────────────────────────
        Commands::Nav { action } => match action {
            NavAction::Parent { selector } => commands::browser::nav_parent(&selector).await,
            NavAction::Children { selector } => commands::browser::nav_children(&selector).await,
            NavAction::NextSibling { selector } => {
                commands::browser::nav_next_sibling(&selector).await
            }
            NavAction::PrevSibling { selector } => {
                commands::browser::nav_prev_sibling(&selector).await
            }
            NavAction::Siblings { selector } => commands::browser::nav_siblings(&selector).await,
            NavAction::Similar { selector } => commands::browser::nav_similar(&selector).await,
            NavAction::Above { selector, limit } => {
                commands::browser::nav_above(&selector, limit).await
            }
            NavAction::Below { selector, limit } => {
                commands::browser::nav_below(&selector, limit).await
            }
        },

        // ── Content Extraction ──────────────────────────────────────
        Commands::Extract { action } => match action {
            ExtractAction::Content {
                format,
                selector,
                output,
            } => {
                commands::browser::extract_content(&format, selector.as_deref(), output.as_deref())
                    .await
            }
            ExtractAction::Metadata => commands::browser::extract_metadata().await,
        },
    }
}
