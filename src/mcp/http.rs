use crate::mcp::server::new_server;
use axum::Router;
use color_eyre::eyre::{Result, eyre};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::net::SocketAddr;

pub async fn serve_http(listen: SocketAddr, path: &str) -> Result<()> {
    let path = normalized_path(path);
    let service: StreamableHttpService<_, LocalSessionManager> = StreamableHttpService::new(
        || Ok(new_server()),
        Default::default(),
        StreamableHttpServerConfig::default().with_sse_keep_alive(None),
    );
    let router = Router::new().nest_service(&path, service);
    let listener = tokio::net::TcpListener::bind(listen).await?;
    let local_addr = listener.local_addr()?;

    eprintln!("Listening on http://{local_addr}{path}");

    axum::serve(listener, router)
        .await
        .map_err(|error| eyre!(error))
}

fn normalized_path(path: &str) -> String {
    if path.is_empty() || path == "/" {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}
