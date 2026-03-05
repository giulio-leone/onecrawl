use clap::Subcommand;


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
pub(crate) enum WindowAction {
    /// Open a new browser window
    New,
}


#[derive(Subcommand)]
pub(crate) enum SetAction {
    /// Set viewport size
    Viewport {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
    },
    /// Emulate a device
    Device {
        /// Device name (e.g., "iPhone 14")
        name: String,
    },
    /// Set geolocation
    Geo {
        /// Latitude
        lat: f64,
        /// Longitude
        lng: f64,
    },
    /// Toggle offline mode
    Offline {
        /// on or off
        #[arg(default_value = "on")]
        state: String,
    },
    /// Set extra HTTP headers (JSON object)
    Headers {
        /// JSON object of headers
        json: String,
    },
    /// Set HTTP basic auth credentials
    Credentials {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// Set color scheme (dark, light, no-preference)
    Media {
        /// Color scheme
        #[arg(default_value = "dark")]
        scheme: String,
    },
}

