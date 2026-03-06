use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum SkillsAction {
    /// List available skills (built-in and discovered)
    List,
    /// Show details about a specific skill
    Info {
        /// Skill name
        name: String,
    },
    /// Discover skills from a directory
    Discover {
        /// Path to scan for skill packages
        path: String,
    },
}
