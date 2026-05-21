mod concepts;
mod generate;
mod init;
mod run;
mod schema;
mod support;
mod validate;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use concepts::ConceptsCommand;
use generate::GenerateCommand;
use init::InitCommand;
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

    /// Repository directory for commands that operate on a Takt repository
    #[arg(long, global = true, value_name = "PATH")]
    repo_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let context = CommandContext {
            format: self.format,
            repo_dir: self.repo_dir,
        };
        match self.command {
            Command::Concepts(command) => command.run(context),
            Command::Init(command) => command.run(context),
            Command::Generate(command) => command.run(context),
            Command::Schema(command) => command.run(context),
            Command::Validate(command) => command.run(context),
            Command::Run(command) => command.run(context),
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
    /// Validate package, action, and workflow manifests
    Validate(ValidateCommand),
    /// Plan a package action or workflow run
    Run(RunCommand),
}
