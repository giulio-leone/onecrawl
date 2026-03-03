use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "onecrawl", version, about = "OneCrawl — AI-native browser automation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Browser session management
    Session {
        #[command(subcommand)]
        action: commands::session::SessionAction,
    },
    /// Navigate to a URL
    Navigate {
        /// Target URL
        url: String,
        /// Wait for page load (ms)
        #[arg(short, long, default_value = "0")]
        wait: u64,
    },
    /// Click an element
    Click {
        /// CSS selector
        selector: String,
    },
    /// Type text into an element
    Type {
        /// CSS selector
        selector: String,
        /// Text to type
        text: String,
    },
    /// Take a screenshot
    Screenshot {
        /// Output file path
        #[arg(short, long, default_value = "screenshot.png")]
        output: String,
        /// Full page screenshot
        #[arg(short, long)]
        full: bool,
    },
    /// Get page accessibility tree
    Snapshot {
        /// Interactive elements only
        #[arg(short, long)]
        interactive: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Get page content
    Get {
        /// What to get: text, html, url, title
        what: String,
        /// CSS selector (for text/html)
        selector: Option<String>,
    },
    /// Evaluate JavaScript
    Eval {
        /// JavaScript expression
        expression: String,
    },
    /// Crypto operations
    Crypto {
        #[command(subcommand)]
        action: commands::crypto::CryptoAction,
    },
    /// Parse HTML
    Parse {
        #[command(subcommand)]
        action: commands::parse::ParseAction,
    },
    /// Storage operations
    Storage {
        #[command(subcommand)]
        action: commands::storage::StorageAction,
    },
    /// Wait for condition
    Wait {
        /// Wait type: element, load, ms
        what: String,
        /// Value (selector, url, or milliseconds)
        value: String,
    },
    /// Save page as PDF
    Pdf {
        /// Output file path
        #[arg(short, long, default_value = "page.pdf")]
        output: String,
    },
    /// Focus an element
    Focus {
        /// CSS selector
        selector: String,
    },
    /// Hover over an element
    Hover {
        /// CSS selector
        selector: String,
    },
    /// Scroll element into view
    ScrollIntoView {
        /// CSS selector
        selector: String,
    },
    /// Double-click an element
    Dblclick {
        /// CSS selector
        selector: String,
    },
    /// Highlight an element
    Highlight {
        /// CSS selector
        selector: String,
        /// Highlight color
        #[arg(short, long, default_value = "red")]
        color: String,
        /// Duration in ms
        #[arg(short, long, default_value = "2000")]
        duration: u64,
    },
    /// Health check
    Health,
    /// Show version and system info
    Info,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Health => {
            println!("✅ OneCrawl Rust CLI v{}", env!("CARGO_PKG_VERSION"));
            println!("   Crates: core, crypto, parser, storage, cdp");
            println!("   Runtime: Tokio async");
        }
        Commands::Info => {
            println!("OneCrawl v{}", env!("CARGO_PKG_VERSION"));
            println!("Arch: {}", std::env::consts::ARCH);
            println!("OS: {}", std::env::consts::OS);
            println!("Rust: compiled native binary");
        }
        Commands::Crypto { action } => commands::crypto::handle(action),
        Commands::Parse { action } => commands::parse::handle(action),
        Commands::Storage { action } => commands::storage::handle(action).await,
        _ => {
            eprintln!("⚠️  Browser commands require an active session. Use `onecrawl session start` first.");
            std::process::exit(1);
        }
    }
}
