pub mod cdp_tools;
pub mod helpers;
pub mod types;
pub mod server;

pub use server::OneCrawlMcp;

/// Start MCP server on stdio transport (blocking until shutdown).
pub async fn start_stdio(
    store_path: String,
    store_password: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mcp = OneCrawlMcp::new(store_path, store_password);
    tracing::info!("starting OneCrawl MCP server (stdio transport)");
    let service = rmcp::ServiceExt::serve(mcp, rmcp::transport::stdio()).await?;
    tokio::select! {
        result = service.waiting() => { result?; }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal, stopping...");
        }
    }
    Ok(())
}
