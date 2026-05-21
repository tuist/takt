use crate::cli::support::{CommandContext, OutputFormat, print_json, print_written_files};
use crate::core;
use clap::{Args, Subcommand};
use color_eyre::eyre::Result;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub(crate) struct GenerateCommand {
    #[command(subcommand)]
    command: GenerateSubcommand,
}

impl GenerateCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        match self.command {
            GenerateSubcommand::Action(command) => command.run(context),
            GenerateSubcommand::Workflow(command) => command.run(context),
        }
    }
}

#[derive(Debug, Subcommand)]
enum GenerateSubcommand {
    /// Generate a starter action manifest
    Action(GenerateActionCommand),
    /// Generate a starter workflow manifest
    Workflow(GenerateWorkflowCommand),
}

#[derive(Debug, Args)]
struct GenerateActionCommand {
    /// Action name
    name: String,
    /// Capability reference this action uses
    capability: String,
    /// Output path for the action manifest
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Overwrite an existing file
    #[arg(long)]
    force: bool,
}

impl GenerateActionCommand {
    fn run(self, context: CommandContext) -> Result<()> {
        let output = core::generate_action(self.name, self.capability, self.output, self.force)?;

        match context.format {
            OutputFormat::Text => {
                print_written_files(&output.files);
                Ok(())
            }
            OutputFormat::Json => print_json(&output),
        }
    }
}

#[derive(Debug, Args)]
struct GenerateWorkflowCommand {
    /// Workflow name
    name: String,
    /// Action reference used by the starter step
    #[arg(long, default_value = "example-action")]
    uses: String,
    /// Output path for the workflow manifest
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Overwrite an existing file
    #[arg(long)]
    force: bool,
}

impl GenerateWorkflowCommand {
    fn run(self, context: CommandContext) -> Result<()> {
        let output = core::generate_workflow(self.name, self.uses, self.output, self.force)?;

        match context.format {
            OutputFormat::Text => {
                print_written_files(&output.files);
                Ok(())
            }
            OutputFormat::Json => print_json(&output),
        }
    }
}
