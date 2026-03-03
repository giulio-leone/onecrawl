mod server;

use clap::Parser;

#[derive(Parser)]
#[command(name = "onecrawl-mcp", about = "OneCrawl MCP Server")]
struct Cli {
    /// Transport mode: "stdio" or "sse"
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// Port for SSE transport
    #[arg(long, default_value = "3001")]
    port: u16,

    /// Path for the encrypted key-value store
    #[arg(long, default_value = "/tmp/onecrawl-mcp-store")]
    store_path: String,

    /// Password for the encrypted store
    #[arg(long, default_value = "onecrawl-default-key")]
    store_password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let mcp = server::OneCrawlMcp::new(cli.store_path, cli.store_password);

    match cli.transport.as_str() {
        "stdio" => {
            tracing::info!("starting OneCrawl MCP server (stdio transport)");
            let service = rmcp::ServiceExt::serve(mcp, rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }
        "sse" => {
            eprintln!("SSE transport on port {} (not yet implemented)", cli.port);
            std::process::exit(1);
        }
        other => {
            eprintln!("unknown transport: {other}");
            std::process::exit(1);
        }
    }

    Ok(())
}
