use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum AndroidAction {
    /// List connected Android devices via ADB
    Devices,
    /// Start an Android automation session via UIAutomator2
    Connect {
        /// UIAutomator2 server URL
        #[arg(long, default_value = "http://localhost:4723")]
        server_url: String,
        /// Device serial (auto-detect if omitted)
        #[arg(long)]
        serial: Option<String>,
        /// Package to automate
        #[arg(long, default_value = "com.android.chrome")]
        package: String,
        /// Activity to launch
        #[arg(long)]
        activity: Option<String>,
    },
    /// Navigate to a URL in Chrome
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
    /// Swipe between two points
    Swipe {
        /// Start X
        from_x: f64,
        /// Start Y
        from_y: f64,
        /// End X
        to_x: f64,
        /// End Y
        to_y: f64,
        /// Duration in milliseconds
        #[arg(long, default_value = "500")]
        duration: u64,
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
    /// Pinch gesture (zoom in/out)
    Pinch {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
        /// Scale factor (>1 zoom in, <1 zoom out)
        scale: f64,
    },
    /// Type text into focused element
    Type {
        /// Text to type
        text: String,
    },
    /// Find element by locator strategy
    Find {
        /// Locator strategy (id, xpath, accessibility id, class name)
        strategy: String,
        /// Locator value
        value: String,
    },
    /// Click element by element ID
    Click {
        /// Element ID
        element_id: String,
    },
    /// Take a screenshot and save to file
    Screenshot {
        /// Output file path
        output: String,
    },
    /// Get or set device orientation
    Orientation {
        /// Set orientation (PORTRAIT/LANDSCAPE). Omit to get current.
        #[arg(long)]
        set: Option<String>,
    },
    /// Press a hardware key by keycode
    Key {
        /// Android keycode (3=HOME, 4=BACK, 24=VOL_UP, 25=VOL_DOWN, 26=POWER)
        keycode: i32,
    },
    /// Launch an app by package name
    AppLaunch {
        /// Package name
        package: String,
        /// Activity name
        #[arg(long)]
        activity: Option<String>,
    },
    /// Kill an app by package name
    AppKill {
        /// Package name
        package: String,
    },
    /// Get app state by package name
    AppState {
        /// Package name
        package: String,
    },
    /// Install an APK
    Install {
        /// Path to APK file
        apk_path: String,
    },
    /// Execute JavaScript in Chrome context
    Script {
        /// JavaScript code
        script: String,
    },
    /// Run ADB shell command
    Shell {
        /// Device serial
        serial: String,
        /// Shell command
        command: String,
    },
    /// Push a file to device
    Push {
        /// Device serial
        serial: String,
        /// Local file path
        local: String,
        /// Remote path on device
        remote: String,
    },
    /// Pull a file from device
    Pull {
        /// Device serial
        serial: String,
        /// Remote path on device
        remote: String,
        /// Local file path
        local: String,
    },
    /// Get device info via ADB
    Info {
        /// Device serial
        serial: String,
    },
    /// Get battery info via ADB
    Battery {
        /// Device serial
        serial: String,
    },
    /// Close the Android session
    Disconnect,
    /// Get current page URL (Chrome)
    Url,
    /// Get current page title (Chrome)
    Title,
}
