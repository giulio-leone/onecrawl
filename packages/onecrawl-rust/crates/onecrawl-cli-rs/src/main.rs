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
        Commands::Type { selector, text } => {
            commands::browser::type_text(&selector, &text).await
        }
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
        Commands::BoundingBox { selector } => {
            commands::browser::bounding_box(&selector).await
        }

        // ── Keyboard ────────────────────────────────────────────────
        Commands::PressKey { key } => commands::browser::press_key(&key).await,
        Commands::KeyDown { key } => commands::browser::key_down(&key).await,
        Commands::KeyUp { key } => commands::browser::key_up(&key).await,
        Commands::KeyboardShortcut { keys } => {
            commands::browser::keyboard_shortcut(&keys).await
        }

        // ── Screenshot / PDF ────────────────────────────────────────
        Commands::Screenshot {
            output,
            full,
            element,
            format,
            quality,
        } => {
            commands::browser::screenshot(&output, full, element.as_deref(), &format, quality)
                .await
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
            EmulateAction::UserAgent { ua } => {
                commands::browser::emulate_user_agent(&ua).await
            }
            EmulateAction::Geolocation {
                lat,
                lon,
                accuracy,
            } => commands::browser::emulate_geolocation(lat, lon, accuracy).await,
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
    }
}
