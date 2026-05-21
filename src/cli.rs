mod concepts;
mod generate;
mod init;
mod mcp;
mod run;
mod schema;
mod support;
mod validate;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use concepts::ConceptsCommand;
use generate::GenerateCommand;
use init::InitCommand;
use mcp::McpCommand;
use run::RunCommand;
use schema::SchemaCommand;
use std::path::PathBuf;
use support::{CommandContext, OutputFormat};
use validate::ValidateCommand;

#[derive(Debug, Parser)]
#[command(
    name = "takt",
    about = "Composable workflows for AI agents",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Output format for command responses
    #[arg(long, value_enum, global = true, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Package directory for commands that operate on a Takt package
    #[arg(long = "package-dir", global = true, value_name = "PATH")]
    package_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let context = CommandContext {
            format: self.format,
            package_dir: self.package_dir,
        };
        match self.command {
            Command::Concepts(command) => command.run(context),
            Command::Init(command) => command.run(context),
            Command::Generate(command) => command.run(context),
            Command::Schema(command) => command.run(context),
            Command::Validate(command) => command.run(context),
            Command::Run(command) => command.run(context),
            Command::Mcp(command) => command.run(context),
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the canonical Takt object model
    Concepts(ConceptsCommand),
    /// Initialize a Takt package
    Init(InitCommand),
    /// Generate Takt actions and workflows
    #[command(visible_alias = "g")]
    Generate(GenerateCommand),
    /// Emit machine-readable schemas for Takt domain objects
    Schema(SchemaCommand),
    /// Validate package, action, and workflow manifests
    Validate(ValidateCommand),
    /// Plan a package action or workflow run
    Run(RunCommand),
    /// Start the Takt MCP server
    Mcp(McpCommand),
}
