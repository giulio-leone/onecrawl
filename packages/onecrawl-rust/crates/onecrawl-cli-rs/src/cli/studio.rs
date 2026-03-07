use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum StudioAction {
    /// Start studio server and open browser
    Open {
        /// Port for the studio server
        #[arg(long, default_value = "9100")]
        port: u16,
    },
    /// List available workflow templates
    Templates,
    /// List saved projects
    Projects,
    /// Export a project as workflow JSON
    Export {
        /// Project ID to export
        project_id: String,
        /// Output file path
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Import a workflow JSON as a new project
    Import {
        /// Path to the workflow JSON file
        file: String,
        /// Name for the imported project
        #[arg(long)]
        name: Option<String>,
    },
    /// Validate a workflow JSON file
    Validate {
        /// Path to the workflow JSON file
        file: String,
    },
}
