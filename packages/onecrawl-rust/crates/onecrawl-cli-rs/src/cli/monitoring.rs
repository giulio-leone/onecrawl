use clap::Subcommand;


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

