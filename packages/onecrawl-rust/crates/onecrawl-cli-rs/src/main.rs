use clap::Parser;

mod cli;
mod commands;
mod dispatch;

use cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    dispatch::dispatch(cli.command).await;
}
