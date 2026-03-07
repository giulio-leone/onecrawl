use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum PluginCliAction {
    /// Install a plugin from a local directory
    Install {
        /// Path to the plugin directory (must contain plugin.json)
        path: String,
    },
    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        name: String,
    },
    /// Enable a plugin
    Enable {
        /// Plugin name
        name: String,
    },
    /// Disable a plugin
    Disable {
        /// Plugin name
        name: String,
    },
    /// List all installed plugins
    List,
    /// Get detailed info about a plugin
    Info {
        /// Plugin name
        name: String,
    },
    /// Create a new plugin scaffold
    Create {
        /// Plugin name (alphanumeric + hyphens/underscores)
        name: String,
        /// Directory where the scaffold will be created
        #[arg(long)]
        path: Option<String>,
    },
    /// Execute a plugin action
    Run {
        /// Plugin name
        plugin_name: String,
        /// Action name
        action_name: String,
        /// JSON parameters
        #[arg(long)]
        params: Option<String>,
    },
    /// Set plugin configuration
    Config {
        /// Plugin name
        name: String,
        /// JSON configuration (key=value or JSON string)
        #[arg(long)]
        set: Option<String>,
    },
}
