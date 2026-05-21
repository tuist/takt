use crate::cli::support::CommandContext;
use crate::mcp::{http, server};
use clap::{Args, ValueEnum};
use color_eyre::eyre::Result;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TransportMode {
    Stdio,
    Http,
}

#[derive(Debug, Args)]
pub(crate) struct McpCommand {
    /// Transport to expose for the MCP server
    #[arg(short, long, env = "TAKT_MCP_TRANSPORT", value_enum, default_value_t = TransportMode::Stdio)]
    transport: TransportMode,
    /// Address to bind when running the HTTP transport
    #[arg(short, long, env = "TAKT_MCP_LISTEN", default_value = "127.0.0.1:0")]
    listen: SocketAddr,
    /// HTTP path to mount the MCP endpoint on
    #[arg(short, long, env = "TAKT_MCP_PATH", default_value = "/mcp")]
    path: String,
}

impl McpCommand {
    pub(crate) fn run(self, _context: CommandContext) -> Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(async move {
                match self.transport {
                    TransportMode::Stdio => server::serve_stdio().await,
                    TransportMode::Http => http::serve_http(self.listen, &self.path).await,
                }
            })
    }
}
