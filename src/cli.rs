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

#[derive(Debug, Parser)]
#[command(
    name = "takt",
    about = "Composable workflows for AI agents",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Concepts(command) => command.run(),
            Command::Init(command) => command.run(),
            Command::Generate(command) => command.run(),
            Command::Schema(command) => command.run(),
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
