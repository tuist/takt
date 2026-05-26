use crate::cli::support::{CommandContext, print_data};
use crate::core;
use crate::datastore::{RunStatus, RunKind, RunRecord, RunSource};
use clap::{Args, Subcommand, ValueEnum};
use color_eyre::eyre::{Result, bail};
use schemars::JsonSchema;
use serde::Serialize;
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
            core::discover_repository(context.package_dir.clone().unwrap_or(env::current_dir()?))?;

        match self.command {
            RunSubcommand::Action(command) => {
                let parsed_inputs = core::parse_input_assignments(&command.input)?;
                let persist = !command.no_persist;
                let output = if command.plan_only {
                    core::plan_action_run(
                        &repo,
                        &command.selector,
                        parsed_inputs,
                        persist,
                        RunSource::Cli,
                    )?
                } else {
                    core::execute_action_run(
                        &repo,
                        &command.selector,
                        parsed_inputs,
                        persist,
                        RunSource::Cli,
                    )?
                };
                print_data(&output, context.format)
            }
            RunSubcommand::Workflow(command) => {
                let parsed_inputs = core::parse_input_assignments(&command.input)?;
                let persist = !command.no_persist;
                let output = if command.plan_only {
                    core::plan_workflow_run(
                        &repo,
                        &command.selector,
                        parsed_inputs,
                        persist,
                        RunSource::Cli,
                    )?
                } else {
                    core::execute_workflow_run(
                        &repo,
                        &command.selector,
                        parsed_inputs,
                        persist,
                        RunSource::Cli,
                    )?
                };
                print_data(&output, context.format)
            }
            RunSubcommand::List(command) => {
                let input = core::RunListInput {
                    kind: command.kind.map(Into::into),
                    status: command.status.map(Into::into),
                    since: command.since,
                    limit: command.limit,
                    predicates: command.r#where,
                };
                print_data(&core::run_list_envelope(&repo, &input)?, context.format)
            }
            RunSubcommand::Get(command) => match core::get_run(&repo, &command.id)? {
                Some(run) => print_data(
                    &RunGetOutput {
                        command: "run get",
                        run,
                    },
                    context.format,
                ),
                None => bail!("run '{}' was not found in the datastore", command.id),
            },
        }
    }
}

#[derive(Debug, Serialize, JsonSchema)]
struct RunGetOutput {
    command: &'static str,
    run: RunRecord,
}

#[derive(Debug, Subcommand)]
enum RunSubcommand {
    /// Plan an action run by name or path
    Action(RunActionCommand),
    /// Plan a workflow run by name or path
    Workflow(RunWorkflowCommand),
    /// List persisted runs in the datastore
    List(RunListCommand),
    /// Get a single persisted run record by id
    Get(RunGetCommand),
}

#[derive(Debug, Args)]
struct RunActionCommand {
    /// Action name or path
    selector: String,
    /// Input bindings in key=value form
    #[arg(
        short,
        long = "input",
        env = "TAKT_RUN_ACTION_INPUT",
        value_delimiter = ',',
        value_name = "KEY=VALUE"
    )]
    input: Vec<String>,
    /// Do not persist the run to the datastore
    #[arg(short = 'n', long, env = "TAKT_RUN_ACTION_NO_PERSIST")]
    no_persist: bool,
    /// Validate and resolve the action without invoking the handler
    #[arg(long, env = "TAKT_RUN_ACTION_PLAN_ONLY")]
    plan_only: bool,
}

#[derive(Debug, Args)]
struct RunWorkflowCommand {
    /// Workflow name or path
    selector: String,
    /// Input bindings in key=value form
    #[arg(
        short,
        long = "input",
        env = "TAKT_RUN_WORKFLOW_INPUT",
        value_delimiter = ',',
        value_name = "KEY=VALUE"
    )]
    input: Vec<String>,
    /// Do not persist the run to the datastore
    #[arg(short = 'n', long, env = "TAKT_RUN_WORKFLOW_NO_PERSIST")]
    no_persist: bool,
    /// Validate and resolve the workflow without invoking any step handler
    #[arg(long, env = "TAKT_RUN_WORKFLOW_PLAN_ONLY")]
    plan_only: bool,
}

#[derive(Debug, Args)]
struct RunListCommand {
    /// Filter by run kind
    #[arg(long, value_enum)]
    kind: Option<CliRunKind>,
    /// Filter by persisted status
    #[arg(long, value_enum)]
    status: Option<CliRunStatus>,
    /// Only include runs started within the given duration (e.g. 30s, 5m, 2h, 7d)
    #[arg(long, value_name = "DURATION")]
    since: Option<String>,
    /// Maximum number of runs to return (newest first)
    #[arg(long, value_name = "N")]
    limit: Option<usize>,
    /// Equality predicate over a run record path (repeatable): --where source.kind=workflow
    #[arg(long = "where", value_name = "PATH=VALUE")]
    r#where: Vec<String>,
}

#[derive(Debug, Args)]
struct RunGetCommand {
    /// Run id
    id: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliRunKind {
    Action,
    Workflow,
}

impl From<CliRunKind> for RunKind {
    fn from(value: CliRunKind) -> Self {
        match value {
            CliRunKind::Action => RunKind::Action,
            CliRunKind::Workflow => RunKind::Workflow,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliRunStatus {
    Planned,
    Running,
    Succeeded,
    Failed,
}

impl From<CliRunStatus> for RunStatus {
    fn from(value: CliRunStatus) -> Self {
        match value {
            CliRunStatus::Planned => RunStatus::Planned,
            CliRunStatus::Running => RunStatus::Running,
            CliRunStatus::Succeeded => RunStatus::Succeeded,
            CliRunStatus::Failed => RunStatus::Failed,
        }
    }
}
