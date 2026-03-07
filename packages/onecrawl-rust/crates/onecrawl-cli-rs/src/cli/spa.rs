use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum SpaAction {
    /// Watch SPA route changes (pushState, replaceState, hashchange)
    NavWatch,
    /// Detect frontend framework and version
    FrameworkDetect,
    /// Detect virtual scroll containers
    VirtualScrollDetect,
    /// Extract items from virtual scroll container
    VirtualScrollExtract {
        /// CSS selector of scroll container
        container: String,
        /// CSS selector of items
        #[arg(long)]
        item: String,
        /// Max items to extract
        #[arg(long, default_value = "1000")]
        max: usize,
    },
    /// Wait for SSR→CSR hydration completion
    WaitHydration {
        /// Timeout in ms
        #[arg(long, default_value = "10000")]
        timeout: u64,
    },
    /// Wait for CSS animations to complete
    WaitAnimation {
        /// CSS selector of animated element
        selector: String,
        /// Timeout in ms
        #[arg(long, default_value = "5000")]
        timeout: u64,
    },
    /// Trigger lazy loading of elements
    TriggerLazyLoad {
        /// CSS selector (default: img[data-src], img[loading='lazy'])
        #[arg(long)]
        selector: Option<String>,
    },
    /// Wait for network to become idle
    WaitNetworkIdle {
        /// Idle duration threshold in ms
        #[arg(long, default_value = "500")]
        idle_ms: u64,
        /// Timeout in ms
        #[arg(long, default_value = "30000")]
        timeout: u64,
    },
    /// Inspect SPA state stores (Redux, Zustand, Pinia)
    StateInspect {
        /// Dot-separated path into store (e.g., "user.profile")
        #[arg(long)]
        path: Option<String>,
    },
    /// Track multi-step form wizard state
    FormWizardTrack,
    /// Wait for dynamic imports / code-split chunks
    DynamicImportWait {
        /// URL pattern to match
        pattern: String,
        /// Timeout in ms
        #[arg(long, default_value = "10000")]
        timeout: u64,
    },
    /// Execute multiple JS expressions in parallel
    ParallelExec {
        /// JS expressions (can specify multiple)
        #[arg(required = true)]
        actions: Vec<String>,
    },
}
