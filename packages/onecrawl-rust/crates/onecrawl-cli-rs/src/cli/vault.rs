use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum VaultAction {
    /// Create a new encrypted vault
    Create {
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Open and verify an existing vault
    Open {
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Store a secret key-value pair
    Set {
        /// Secret key (e.g. "linkedin.email")
        key: String,
        /// Secret value (omit to read from stdin)
        value: Option<String>,
        /// Service category (e.g. "linkedin")
        #[arg(long)]
        category: Option<String>,
        /// Read value from stdin without echo
        #[arg(long)]
        prompt: bool,
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Retrieve a secret value
    Get {
        /// Secret key to retrieve
        key: String,
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Delete a secret
    Delete {
        /// Secret key to delete
        key: String,
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// List all secrets (keys only, no values)
    List {
        /// Filter by service category
        #[arg(long)]
        category: Option<String>,
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Export service credentials as workflow variables
    Use {
        /// Service name (e.g. "linkedin")
        service: String,
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Change master password
    ChangePassword {
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
    /// Import secrets from environment variables
    ImportEnv {
        /// Environment variable prefix
        #[arg(long, default_value = "ONECRAWL_VAULT_")]
        prefix: String,
        /// Path to the vault file
        #[arg(long, default_value = "~/.onecrawl/vault.enc")]
        path: String,
    },
}
