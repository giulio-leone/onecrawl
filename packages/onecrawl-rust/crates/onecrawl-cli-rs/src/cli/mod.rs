use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "onecrawl", version, about = "OneCrawl — AI-native browser automation", long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
mod agent;
mod android;
mod auth;
mod computer;
mod crawl;
mod daemon;
mod dom;
mod durable;
mod harness;
mod interaction;
mod ios;
mod media;
mod monitoring;
mod network;
mod react;
mod skills;
mod spa;
mod storage;
mod streaming_video;
mod tabs;
mod utility;

pub(crate) use agent::AgentCliAction;
pub(crate) use android::AndroidAction;
pub(crate) use auth::{CaptchaAction, AuthAction, StealthAction, AntibotAction, AuthStateAction};
pub(crate) use computer::ComputerCliAction;
pub(crate) use crawl::{PipelineAction, StructuredAction, AdaptiveAction, SpiderAction, RobotsAction, GraphAction};
pub(crate) use daemon::DaemonAction;
pub(crate) use dom::{FingerprintAction, EmulateAction, DomAction, IframeAction, AdvancedEmulationAction, WindowAction, SetAction};
pub(crate) use durable::DurableAction;
pub(crate) use react::ReactAction;
pub(crate) use harness::HarnessAction;
pub(crate) use interaction::{SelectAction, NavAction, KeyboardAction, MouseAction, FindAction};
pub(crate) use ios::IosAction;
pub(crate) use media::{SnapshotAction, ScreenshotDiffAction, PrintAction, ExtractAction, DiffAction};
pub(crate) use monitoring::{CoverageAction, AccessibilityAction, PerfAction, ConsoleAction, DialogAction, WorkerAction, PageWatcherAction};
pub(crate) use network::{DomainAction, HttpAction, NetworkAction, HarAction, WsAction, ThrottleAction, NetworkLogAction, ProxyAction, ProxyHealthAction, InterceptCommandAction};
pub(crate) use skills::SkillsAction;
pub(crate) use spa::SpaAction;
pub(crate) use storage::{CookieJarAction, CookieAction, WebStorageAction};
pub(crate) use streaming_video::{StreamAction, RecordAction};
pub(crate) use tabs::{TabAction, DownloadAction};
pub(crate) use utility::{RateLimitAction, RetryAction, ScheduleAction, PoolAction, BenchAction, GeoAction, RequestAction};


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
    /// Get page content: text, html, url, title, value, attr, count, styles
    Get {
        /// What to get: text, html, url, title, value, attr, count, styles
        what: String,
        /// CSS selector (for text/html/value/attr/count/styles)
        selector: Option<String>,
        /// Extra argument (attribute name for "get attr")
        arg: Option<String>,
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
        /// Annotate interactive elements with numbered labels
        #[arg(short, long)]
        annotate: bool,
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
    /// Wait for text to appear on the page
    WaitForText {
        /// Text to wait for
        text: String,
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },
    /// Wait for a load state (load, domcontentloaded, networkidle)
    WaitForLoad {
        /// Load state: load, domcontentloaded, networkidle
        #[arg(default_value = "networkidle")]
        state: String,
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },
    /// Wait for a JavaScript condition to be true
    WaitForFunction {
        /// JavaScript expression that should return truthy
        expression: String,
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },

    // ── State Checks ────────────────────────────────────────────────
    /// Check element state: visible, enabled, checked
    Is {
        /// State to check: visible, enabled, checked
        check: String,
        /// CSS selector or @ref
        selector: String,
    },

    // ── Scroll ──────────────────────────────────────────────────────
    /// Scroll the page in a direction
    Scroll {
        /// Direction: up, down, left, right
        direction: String,
        /// Pixels to scroll (default: 500)
        #[arg(default_value = "500")]
        pixels: i64,
        /// CSS selector to scroll within (instead of page)
        #[arg(short, long)]
        selector: Option<String>,
    },

    // ── Keyboard (focus-based) ──────────────────────────────────────
    /// Keyboard input at current focus (no selector needed)
    Keyboard {
        #[command(subcommand)]
        action: KeyboardAction,
    },

    // ── Mouse Control ───────────────────────────────────────────────
    /// Low-level mouse control
    Mouse {
        #[command(subcommand)]
        action: MouseAction,
    },

    // ── Find (Semantic Locators) ────────────────────────────────────
    /// Find elements by semantic properties and perform an action
    Find {
        #[command(subcommand)]
        action: FindAction,
    },

    // ── Diff ────────────────────────────────────────────────────────
    /// Compare snapshots, screenshots, or URLs
    Diff {
        #[command(subcommand)]
        action: DiffAction,
    },

    // ── Debug ───────────────────────────────────────────────────────
    /// View page errors (uncaught JavaScript exceptions)
    Errors {
        /// Clear errors instead of viewing
        #[arg(long)]
        clear: bool,
    },
    /// Highlight an element on the page with a visible border
    Highlight {
        /// CSS selector or @ref
        selector: String,
    },

    // ── Auth State Persistence ──────────────────────────────────────
    /// Save/load browser authentication state (cookies + storage)
    AuthState {
        #[command(subcommand)]
        action: AuthStateAction,
    },

    // ── Window ──────────────────────────────────────────────────────
    /// Browser window management
    Window {
        #[command(subcommand)]
        action: WindowAction,
    },

    // ── Set (Browser Config) ────────────────────────────────────────
    /// Set browser configuration
    Set {
        #[command(subcommand)]
        action: SetAction,
    },

    // ── Route (request interception with response mocking) ─────────
    /// Intercept requests matching a pattern and respond with custom data
    Route {
        /// URL pattern to match (glob: `**/*.png`, or regex with `/`)
        pattern: String,
        /// HTTP status code to respond with
        #[arg(long, default_value = "200")]
        status: u16,
        /// Response body (string)
        #[arg(long)]
        body: Option<String>,
        /// Response content-type
        #[arg(long, default_value = "text/plain")]
        content_type: String,
        /// Block matching requests instead of responding
        #[arg(long)]
        block: bool,
    },

    /// Remove a previously set route
    Unroute {
        /// URL pattern to unroute (or "all" to clear everything)
        pattern: String,
    },

    /// List recent network requests (quick inspection)
    Requests {
        /// Filter by URL substring
        #[arg(long)]
        filter: Option<String>,
        /// Max number of requests to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Show only failed requests (4xx/5xx)
        #[arg(long)]
        failed: bool,
    },

    /// Close the current page or browser session
    Close {
        /// Close all pages (entire browser session)
        #[arg(long)]
        all: bool,
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

    // ── Daemon ──────────────────────────────────────────────────
    /// Persistent browser daemon (sub-100ms IPC)
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    // ── Durable Sessions ───────────────────────────────────────
    /// Crash-resilient browser sessions with auto-checkpoint and reconnect
    Durable {
        #[command(subcommand)]
        action: DurableAction,
    },

    // ── Event Reactor ────────────────────────────────────────
    /// Real-time event reactor with configurable handlers
    React {
        #[command(subcommand)]
        action: ReactAction,
    },

    // ── Skills ──────────────────────────────────────────────────
    /// Manage reusable browser automation skill packages
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },

    // ── Live Streaming ───────────────────────────────────────────
    /// Live browser screencast via CDP
    Stream {
        #[command(subcommand)]
        action: StreamAction,
    },

    // ── Video Recording ─────────────────────────────────────────
    /// Record browser session as frame sequence
    Record {
        #[command(subcommand)]
        action: RecordAction,
    },

    // ── iOS / Mobile Safari ────────────────────────────────────
    /// iOS Safari automation via WebDriverAgent
    Ios {
        #[command(subcommand)]
        action: IosAction,
    },

    // ── Android / ADB + UIAutomator2 ──────────────────────────
    /// Android automation via ADB + UIAutomator2
    Android {
        #[command(subcommand)]
        action: AndroidAction,
    },

    // ── SPA Interaction ─────────────────────────────────────────
    /// SPA interaction commands (hydration, virtual scroll, state)
    Spa {
        #[command(subcommand)]
        action: SpaAction,
    },

    // ── Harness ─────────────────────────────────────────────────
    /// Long-running harness commands (health, circuit breaker, watchdog)
    Harness {
        #[command(subcommand)]
        action: HarnessAction,
    },

    // ── Agentic AI ──────────────────────────────────────────────
    /// Agentic AI commands (loop, goals, reasoning)
    #[command(subcommand)]
    Agent(AgentCliAction),

    // ── Computer Use ────────────────────────────────────────────
    /// Computer use commands (screenshots, clicks, replay)
    #[command(subcommand)]
    Computer(ComputerCliAction),

    // ── Enhanced Agentic ────────────────────────────────────────
    /// Get compact page state for AI agents
    PageState,
    /// Execute multi-step JS plan
    PlanExec {
        #[arg(required = true)]
        steps: Vec<String>,
    },
    /// Get page summary optimized for AI
    PageInfo,
    /// Check multiple page assertions (format: "type:expected")
    Assert {
        /// Assertions in "type:expected" format (e.g., "url_contains:dashboard")
        #[arg(required = true)]
        checks: Vec<String>,
    },
    /// Get detailed element information
    ElementDetail {
        /// CSS selector of the element to inspect
        selector: String,
    },

    // ── Workflow ────────────────────────────────────────────────────
    /// Execute a workflow from a JSON file
    WorkflowExec {
        /// Path to workflow JSON file
        file: String,
    },
    /// Validate a workflow JSON file
    WorkflowValidate {
        /// Path to workflow JSON file
        file: String,
    },
    /// Resume a paused workflow after agent decision
    WorkflowResume {
        /// Path to workflow JSON file
        file: String,
        /// Step index to resume from
        #[arg(long)]
        resume_from: usize,
        /// Agent's chosen action
        #[arg(long)]
        choice: String,
        /// Reasoning behind the decision
        #[arg(long)]
        reasoning: Option<String>,
    },
    /// Present a decision prompt to an AI agent
    AgentDecide {
        /// The prompt or question for the agent
        prompt: String,
        /// Available options (comma-separated)
        #[arg(long)]
        options: Option<String>,
    },

    // ── Generic MCP Action Runner ──────────────────────────────────
    /// Run any MCP action: onecrawl run <tool> <action> --json '{...}'
    Run {
        /// Tool namespace (browser, crawl, agent, stealth, data, secure, computer, memory, automate, perf)
        tool: String,
        /// Action name within the tool
        action: String,
        /// JSON parameters (optional)
        #[arg(long)]
        json: Option<String>,
    },
    /// List all available MCP tools and actions
    RunList,

    // ── Version ─────────────────────────────────────────────────
    /// Show version and build information
    Version,
}

