use clap::Subcommand;

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the daemon (spawns a background process)
    Start {
        /// Run browser in headless mode
        #[arg(long)]
        headless: bool,
    },
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
    /// Send a command to the daemon
    Exec {
        /// Command name (e.g. goto, click, evaluate, ping)
        command: String,
        /// Command arguments as key=value pairs
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        /// Target a specific named session
        #[arg(short, long)]
        session: Option<String>,
    },
    /// Run the daemon in the foreground (used internally by `start`)
    #[command(hide = true)]
    Run {
        /// Run browser in headless mode
        #[arg(long)]
        headless: bool,
    },
}
