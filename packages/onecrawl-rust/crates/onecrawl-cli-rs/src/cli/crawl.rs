use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum PipelineAction {
    /// Run a pipeline on data
    Run {
        /// Path to pipeline definition JSON
        pipeline_json: String,
        /// Path to data JSON file (array of objects)
        data_json: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
        /// Output format: json, jsonl, csv
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Validate a pipeline definition
    Validate {
        /// Path to pipeline definition JSON
        pipeline_json: String,
    },
    /// Save a pipeline definition to a file
    Save {
        /// Pipeline definition JSON (inline)
        pipeline_json: String,
        /// Output file path
        path: String,
    },
    /// Load and display a pipeline from a file
    Load {
        /// Input file path
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum StructuredAction {
    /// Extract all structured data from the current page
    ExtractAll,
    /// Extract JSON-LD from the current page
    JsonLd,
    /// Extract OpenGraph metadata from the current page
    OpenGraph,
    /// Extract Twitter Card metadata from the current page
    TwitterCard,
    /// Extract page metadata from the current page
    Metadata,
    /// Validate extracted structured data
    Validate {
        /// JSON string of StructuredDataResult
        data_json: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum AdaptiveAction {
    /// Fingerprint a DOM element by CSS selector
    Fingerprint {
        /// CSS selector
        selector: String,
    },
    /// Relocate an element using a fingerprint JSON
    Relocate {
        /// Fingerprint JSON string
        fingerprint_json: String,
    },
    /// Track multiple elements by selectors (JSON array)
    Track {
        /// JSON array of CSS selectors
        selectors: String,
        /// Optional path to save fingerprints
        #[arg(short, long)]
        save: Option<String>,
    },
    /// Relocate all tracked elements from fingerprints JSON
    RelocateAll {
        /// JSON array of fingerprints
        fingerprints_json: String,
    },
    /// Save fingerprints JSON to a file
    Save {
        /// JSON array of fingerprints
        fingerprints: String,
        /// File path
        path: String,
    },
    /// Load fingerprints from a file
    Load {
        /// File path
        path: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum SpiderAction {
    /// Crawl starting from a URL
    Crawl {
        /// Start URL
        start_url: String,
        /// Maximum crawl depth
        #[arg(long, default_value = "3")]
        max_depth: usize,
        /// Maximum number of pages
        #[arg(long, default_value = "100")]
        max_pages: usize,
        /// Concurrent workers (reserved for future use)
        #[arg(long, default_value = "3")]
        concurrency: usize,
        /// Delay between requests in milliseconds
        #[arg(long, default_value = "500")]
        delay: u64,
        /// Only follow links on the same domain
        #[arg(long, default_value = "true")]
        same_domain: bool,
        /// CSS selector to extract from each page
        #[arg(long)]
        selector: Option<String>,
        /// Content format: text, html, markdown, json
        #[arg(long, default_value = "text")]
        format: String,
        /// Save results to file
        #[arg(long)]
        output: Option<String>,
        /// Output file format: json or jsonl
        #[arg(long, default_value = "json")]
        output_format: String,
    },
    /// Resume a crawl from a saved state file
    Resume {
        /// Path to the state JSON file
        state_file: String,
    },
    /// Print summary of a results file
    Summary {
        /// Path to the results JSON file
        results_file: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum RobotsAction {
    /// Parse robots.txt from a URL or local file
    Parse {
        /// URL or file path to robots.txt
        source: String,
    },
    /// Check if a path is allowed by robots.txt
    Check {
        /// URL to the site (fetches /robots.txt)
        url: String,
        /// Path to check
        path: String,
        /// User-agent string
        #[arg(long, default_value = "*")]
        user_agent: String,
    },
    /// List sitemaps declared in robots.txt
    Sitemaps {
        /// URL to the site (fetches /robots.txt)
        url: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum GraphAction {
    /// Extract links from the current page
    Extract {
        /// Base URL for internal/external classification
        #[arg(long)]
        base_url: Option<String>,
    },
    /// Build a graph from edges JSON file
    Build {
        /// Path to edges JSON file
        edges_json: String,
    },
    /// Analyze a graph JSON file
    Analyze {
        /// Path to graph JSON file
        graph_json: String,
    },
    /// Export graph to a JSON file
    Export {
        /// Path to graph JSON file
        graph_json: String,
        /// Output file path
        output_path: String,
    },
}

