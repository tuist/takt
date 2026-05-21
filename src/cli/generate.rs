use crate::cli::support::{slugify, write_yaml_file};
use crate::domain::{ActionDefinition, WorkflowDefinition};
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
    pub(crate) fn run(self) -> Result<()> {
        match self.command {
            GenerateSubcommand::Action(command) => command.run(),
            GenerateSubcommand::Workflow(command) => command.run(),
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
    fn run(self) -> Result<()> {
        let output = self
            .output
            .unwrap_or_else(|| PathBuf::from(format!("actions/{}.yaml", slugify(&self.name))));
        let action = ActionDefinition::starter(self.name, self.capability);
        write_yaml_file(&action, &output, self.force, "action")
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
    fn run(self) -> Result<()> {
        let output = self
            .output
            .unwrap_or_else(|| PathBuf::from(format!("workflows/{}.yaml", slugify(&self.name))));
        let workflow = WorkflowDefinition::starter(self.name, self.uses);
        write_yaml_file(&workflow, &output, self.force, "workflow")
    }
}
