use crate::cli::support::{CommandContext, print_data};
use crate::core;
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, bail};
use std::env;

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub(crate) struct ValidateCommand {
    #[command(subcommand)]
    command: ValidateSubcommand,
}

impl ValidateCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let repo =
            core::discover_repository(context.package_dir.clone().unwrap_or(env::current_dir()?))?;

        match self.command {
            ValidateSubcommand::Package(_) => finish_report(
                core::validate_package(&repo),
                context,
                "package validation failed",
            ),
            ValidateSubcommand::Action(command) => {
                let action = core::load_action(&repo, &command.selector)?;
                finish_report(
                    core::validate_action_document(&repo, &action),
                    context,
                    "action validation failed",
                )
            }
            ValidateSubcommand::Workflow(command) => {
                let workflow = core::load_workflow(&repo, &command.selector)?;
                finish_report(
                    core::validate_workflow_document(&repo, &workflow),
                    context,
                    "workflow validation failed",
                )
            }
            ValidateSubcommand::All(_) => {
                finish_summary(core::validate_all(&repo)?, context, "validation failed")
            }
        }
    }
}

fn finish_report(
    report: core::ValidationReport,
    context: CommandContext,
    failure_message: &str,
) -> Result<()> {
    let passed = report.passed;
    print_data(&report, context.format)?;
    if passed {
        Ok(())
    } else {
        bail!(failure_message.to_string())
    }
}

fn finish_summary(
    summary: core::ValidationSummary,
    context: CommandContext,
    failure_message: &str,
) -> Result<()> {
    let passed = summary.passed;
    print_data(&summary, context.format)?;
    if passed {
        Ok(())
    } else {
        bail!(failure_message.to_string())
    }
}

#[derive(Debug, Subcommand)]
enum ValidateSubcommand {
    /// Validate the current package manifest
    Package(ValidatePackageCommand),
    /// Validate an action manifest by name or path
    Action(ValidateActionCommand),
    /// Validate a workflow manifest by name or path
    Workflow(ValidateWorkflowCommand),
    /// Validate the package and all local action and workflow manifests
    All(ValidateAllCommand),
}

#[derive(Debug, Args)]
struct ValidatePackageCommand;

#[derive(Debug, Args)]
struct ValidateActionCommand {
    /// Action name or path
    selector: String,
}

#[derive(Debug, Args)]
struct ValidateWorkflowCommand {
    /// Workflow name or path
    selector: String,
}

#[derive(Debug, Args)]
struct ValidateAllCommand;
