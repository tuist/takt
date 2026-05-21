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

#[cfg(test)]
mod tests {
    use super::{normalized_path, serve_http};
    use color_eyre::eyre::{Result, bail};
    use std::net::{SocketAddr, TcpListener};
    use std::process::Command;
    use tokio::{
        net::TcpStream,
        task::JoinHandle,
        time::{Duration, sleep},
    };

    #[test]
    fn normalized_path_defaults_empty_input_to_root() {
        assert_eq!(normalized_path(""), "/");
    }

    #[test]
    fn normalized_path_preserves_root_path() {
        assert_eq!(normalized_path("/"), "/");
    }

    #[test]
    fn normalized_path_preserves_existing_leading_slash() {
        assert_eq!(normalized_path("/mcp"), "/mcp");
    }

    #[test]
    fn normalized_path_adds_leading_slash_when_missing() {
        assert_eq!(normalized_path("mcp"), "/mcp");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn serve_http_responds_on_normalized_path() -> Result<()> {
        let (addr, handle) = spawn_http_server("mcp").await?;
        let response = initialize_over_http(addr, "/mcp").await?;

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("content-type: text/event-stream"));
        assert!(response.to_ascii_lowercase().contains("mcp-session-id:"));
        assert!(response.contains("\"serverInfo\":{\"name\":\"takt\""));

        handle.abort();
        let _ = handle.await;

        Ok(())
    }

    fn reserve_local_addr() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        drop(listener);
        Ok(addr)
    }

    async fn spawn_http_server(path: &str) -> Result<(SocketAddr, JoinHandle<Result<()>>)> {
        let addr = reserve_local_addr()?;
        let path = path.to_string();
        let handle = tokio::spawn(async move { serve_http(addr, &path).await });

        wait_for_server(addr).await?;

        Ok((addr, handle))
    }

    async fn wait_for_server(addr: SocketAddr) -> Result<()> {
        for _ in 0..50 {
            if TcpStream::connect(addr).await.is_ok() {
                return Ok(());
            }

            sleep(Duration::from_millis(20)).await;
        }

        Err(color_eyre::eyre::eyre!(
            "timed out waiting for HTTP server on {addr}"
        ))
    }

    async fn initialize_over_http(addr: SocketAddr, path: &str) -> Result<String> {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
        let url = format!("http://{addr}{path}");
        let output = Command::new("curl")
            .args([
                "-sS",
                "--max-time",
                "2",
                "-D",
                "-",
                "-o",
                "-",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-H",
                "Accept: application/json, text/event-stream",
                "--data",
                body,
                &url,
            ])
            .output()?;

        if !output.status.success() {
            bail!(String::from_utf8_lossy(&output.stderr).into_owned());
        }

        Ok(String::from_utf8(output.stdout)?)
    }
}
