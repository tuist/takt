use crate::cli::support::{CommandContext, print_data};
use crate::core;
use clap::{Args, Subcommand};
use color_eyre::eyre::Result;
use std::env;

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub(crate) struct RunCommand {
    #[command(subcommand)]
    command: RunSubcommand,
}

impl RunCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let repo =
            core::discover_repository(context.repo_dir.clone().unwrap_or(env::current_dir()?))?;

        match self.command {
            RunSubcommand::Action(command) => print_data(
                &core::plan_action_run(
                    &repo,
                    &command.selector,
                    core::parse_input_assignments(&command.input)?,
                    !command.no_persist,
                )?,
                context.format,
            ),
            RunSubcommand::Workflow(command) => print_data(
                &core::plan_workflow_run(
                    &repo,
                    &command.selector,
                    core::parse_input_assignments(&command.input)?,
                    !command.no_persist,
                )?,
                context.format,
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum RunSubcommand {
    /// Plan an action run by name or path
    Action(RunActionCommand),
    /// Plan a workflow run by name or path
    Workflow(RunWorkflowCommand),
}

#[derive(Debug, Args)]
struct RunActionCommand {
    /// Action name or path
    selector: String,
    /// Input bindings in key=value form
    #[arg(long = "input", value_name = "KEY=VALUE")]
    input: Vec<String>,
    /// Do not persist the planned run to .takt/runs/
    #[arg(long)]
    no_persist: bool,
}

#[derive(Debug, Args)]
struct RunWorkflowCommand {
    /// Workflow name or path
    selector: String,
    /// Input bindings in key=value form
    #[arg(long = "input", value_name = "KEY=VALUE")]
    input: Vec<String>,
    /// Do not persist the planned run to .takt/runs/
    #[arg(long)]
    no_persist: bool,
}
