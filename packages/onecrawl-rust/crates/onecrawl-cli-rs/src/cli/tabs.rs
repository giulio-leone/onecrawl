use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum TabAction {
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
pub(crate) enum DownloadAction {
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

