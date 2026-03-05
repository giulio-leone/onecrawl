use clap::Subcommand;


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
        /// Include cursor-interactive elements (divs with onclick, tabindex, cursor:pointer)
        #[arg(short = 'C', long)]
        cursor: bool,
        /// Compact mode: remove empty structural elements
        #[arg(short, long)]
        compact: bool,
        /// Limit tree depth
        #[arg(short, long)]
        depth: Option<usize>,
        /// Scope to a CSS selector
        #[arg(short, long)]
        selector: Option<String>,
    },
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
pub(crate) enum DiffAction {
    /// Compare current snapshot vs last (or baseline file)
    Snapshot {
        /// Path to baseline snapshot file
        #[arg(long)]
        baseline: Option<String>,
        /// Scope to a CSS selector
        #[arg(long)]
        selector: Option<String>,
        /// Compact diff output
        #[arg(long)]
        compact: bool,
    },
    /// Visual pixel diff of screenshots
    Screenshot {
        /// Path to baseline screenshot
        #[arg(long)]
        baseline: String,
        /// Output diff image path
        #[arg(short, long)]
        output: Option<String>,
        /// Color threshold (0-1)
        #[arg(short, long, default_value = "0.1")]
        threshold: f64,
    },
    /// Compare two URLs (snapshot diff)
    Url {
        /// First URL
        url1: String,
        /// Second URL
        url2: String,
        /// Also do visual diff
        #[arg(long)]
        screenshot: bool,
        /// Wait strategy: load, domcontentloaded, networkidle
        #[arg(long, default_value = "load")]
        wait_until: String,
        /// Scope to a CSS selector
        #[arg(long)]
        selector: Option<String>,
    },
}

