mod concepts;
mod generate;
mod init;
mod schema;
mod support;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use concepts::ConceptsCommand;
use generate::GenerateCommand;
use init::InitCommand;
use schema::SchemaCommand;
use support::OutputFormat;

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

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let format = self.format;
        match self.command {
            Command::Concepts(command) => command.run(format),
            Command::Init(command) => command.run(format),
            Command::Generate(command) => command.run(format),
            Command::Schema(command) => command.run(format),
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the canonical Takt object model
    Concepts(ConceptsCommand),
    /// Initialize a Takt package repository
    Init(InitCommand),
    /// Generate Takt actions and workflows
    #[command(visible_alias = "g")]
    Generate(GenerateCommand),
    /// Emit machine-readable schemas for Takt domain objects
    Schema(SchemaCommand),
}
