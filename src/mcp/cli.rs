use crate::mcp::{http, server};
use clap::{Parser, ValueEnum};
use color_eyre::eyre::Result;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TransportMode {
    Stdio,
    Http,
}

#[derive(Debug, Parser)]
#[command(name = "takt-mcp", about = "Takt MCP server")]
pub struct Cli {
    /// Transport to expose for the MCP server
    #[arg(long, value_enum, default_value_t = TransportMode::Stdio)]
    transport: TransportMode,
    /// Address to bind when running the HTTP transport
    #[arg(long, default_value = "127.0.0.1:0")]
    listen: SocketAddr,
    /// HTTP path to mount the MCP endpoint on
    #[arg(long, default_value = "/mcp")]
    path: String,
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.transport {
            TransportMode::Stdio => server::serve_stdio().await,
            TransportMode::Http => http::serve_http(self.listen, &self.path).await,
        }
    }
}
