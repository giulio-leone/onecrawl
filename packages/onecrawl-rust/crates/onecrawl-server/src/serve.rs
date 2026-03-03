use std::sync::Arc;

use crate::routes::create_router;
use crate::state::ServerState;

/// Start the OneCrawl HTTP server on the given port.
pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(ServerState::new(port));
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("OneCrawl server listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
