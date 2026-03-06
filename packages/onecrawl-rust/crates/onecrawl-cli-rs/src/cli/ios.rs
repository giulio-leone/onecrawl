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
}
