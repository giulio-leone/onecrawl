use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum IosAction {
    /// List available iOS devices/simulators
    Devices,
    /// Start an iOS Safari session via WebDriverAgent
    Connect {
        /// WebDriverAgent URL
        #[arg(long, default_value = "http://localhost:8100")]
        wda_url: String,
        /// Device UDID (auto-detect if omitted)
        #[arg(long)]
        udid: Option<String>,
    },
    /// Navigate to a URL in Mobile Safari
    Navigate {
        /// Target URL
        url: String,
    },
    /// Tap at screen coordinates
    Tap {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },
    /// Take a screenshot and save to file
    Screenshot {
        /// Output file path
        output: String,
    },
    /// Close the iOS session
    Disconnect,
    /// Pinch gesture (zoom in/out)
    Pinch {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
        /// Scale factor (>1 zoom in, <1 zoom out)
        scale: f64,
        /// Pinch velocity
        #[arg(long, default_value = "1.0")]
        velocity: f64,
    },
    /// Long press at coordinates
    LongPress {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
        /// Duration in milliseconds
        #[arg(long, default_value = "1000")]
        duration: u64,
    },
    /// Double tap at coordinates
    DoubleTap {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },
    /// Get or set device orientation
    Orientation {
        /// Set orientation (PORTRAIT/LANDSCAPE). Omit to get current.
        #[arg(long)]
        set: Option<String>,
    },
    /// Launch an app by bundle ID
    AppLaunch {
        /// Bundle ID
        bundle_id: String,
    },
    /// Kill an app by bundle ID
    AppKill {
        /// Bundle ID
        bundle_id: String,
    },
    /// Get app state by bundle ID
    AppState {
        /// Bundle ID
        bundle_id: String,
    },
    /// Lock the device
    Lock,
    /// Unlock the device
    Unlock,
    /// Press the home button
    Home,
    /// Press a hardware button (e.g. volumeUp, volumeDown)
    Button {
        /// Button name
        name: String,
    },
    /// Get battery info
    Battery,
    /// Get device info
    Info,
    /// Manage iOS simulators (list/boot/shutdown/create/delete)
    Simulator {
        /// Action: list, boot, shutdown, create, delete
        action: String,
        /// Device UDID (required for boot/shutdown/delete)
        #[arg(long)]
        udid: Option<String>,
        /// Device type for create
        #[arg(long)]
        device_type: Option<String>,
        /// Runtime for create
        #[arg(long)]
        runtime: Option<String>,
    },
    /// Get current page URL (Safari)
    Url,
    /// Get current page title (Safari)
    Title,
    /// Execute JavaScript in Safari
    Script {
        /// JavaScript code
        script: String,
    },
    /// Get all cookies (Safari)
    Cookies,
}
