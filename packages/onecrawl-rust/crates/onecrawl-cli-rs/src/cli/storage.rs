use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum CookieJarAction {
    /// Export all cookies to stdout or file
    Export {
        /// Output file path (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import cookies from a file
    Import {
        /// Cookie jar JSON file path
        path: String,
    },
    /// Clear all cookies
    Clear,
}


#[derive(Subcommand)]
pub(crate) enum CookieAction {
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
    /// Export all current page cookies to a JSON file (compatible with --import-cookies)
    Export {
        /// Output file path (defaults to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import cookies from a JSON file (format produced by 'cookie export')
    Import {
        /// Path to the JSON cookie file
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum WebStorageAction {
    /// Get all localStorage contents (JSON)
    LocalGet,
    /// Set a localStorage item
    LocalSet {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Clear all localStorage
    LocalClear,
    /// Get all sessionStorage contents (JSON)
    SessionGet,
    /// Set a sessionStorage item
    SessionSet {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Clear all sessionStorage
    SessionClear,
    /// List IndexedDB database names
    IndexeddbList,
    /// Clear all site data (localStorage + sessionStorage + cookies + cache)
    ClearAll,
}

