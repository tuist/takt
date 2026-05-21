use crate::cli::support::{
    CommandContext, OutputFormat, structured_json_string, written_files_summary,
};
use crate::core;
use clap::{Args, Subcommand};
use color_eyre::eyre::Result;
use serde::Serialize;
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
    #[arg(short, long, env = "TAKT_GENERATE_ACTION_OUTPUT", value_name = "PATH")]
    output: Option<PathBuf>,
    /// Overwrite an existing file
    #[arg(short, long, env = "TAKT_GENERATE_ACTION_FORCE")]
    force: bool,
}

impl GenerateActionCommand {
    fn run(self, context: CommandContext) -> Result<()> {
        let output = core::generate_action(self.name, self.capability, self.output, self.force)?;
        print!(
            "{}",
            render_scaffold_output(&output, &output.files, context.format)?
        );
        Ok(())
    }
}

#[derive(Debug, Args)]
struct GenerateWorkflowCommand {
    /// Workflow name
    name: String,
    /// Action reference used by the starter step
    #[arg(
        short,
        long,
        env = "TAKT_GENERATE_WORKFLOW_USES",
        default_value = "example-action"
    )]
    uses: String,
    /// Output path for the workflow manifest
    #[arg(
        short,
        long,
        env = "TAKT_GENERATE_WORKFLOW_OUTPUT",
        value_name = "PATH"
    )]
    output: Option<PathBuf>,
    /// Overwrite an existing file
    #[arg(short, long, env = "TAKT_GENERATE_WORKFLOW_FORCE")]
    force: bool,
}

impl GenerateWorkflowCommand {
    fn run(self, context: CommandContext) -> Result<()> {
        let output = core::generate_workflow(self.name, self.uses, self.output, self.force)?;
        print!(
            "{}",
            render_scaffold_output(&output, &output.files, context.format)?
        );
        Ok(())
    }
}

fn render_scaffold_output<T>(
    output: &T,
    files: &[crate::core::WrittenFile],
    format: OutputFormat,
) -> Result<String>
where
    T: Serialize,
{
    match format {
        OutputFormat::Text => Ok(written_files_summary(files)),
        OutputFormat::Json | OutputFormat::Toon => structured_json_string(output, format),
    }
}

#[cfg(test)]
mod tests {
    use super::render_scaffold_output;
    use crate::cli::support::OutputFormat;
    use crate::core::{ActionGenerateOutput, WorkflowGenerateOutput, WrittenFile};
    use crate::domain::{ActionDefinition, WorkflowDefinition};
    use color_eyre::eyre::Result;
    use std::path::PathBuf;

    #[test]
    fn action_text_output_matches_snapshot() -> Result<()> {
        let output = sample_action_output();
        insta::assert_snapshot!(render_scaffold_output(&output, &output.files, OutputFormat::Text)?, @r#"
        Wrote actions/github-triage.json
        "#);
        Ok(())
    }

    #[test]
    fn action_toon_output_matches_snapshot() -> Result<()> {
        let output = sample_action_output();
        insta::assert_snapshot!(render_scaffold_output(&output, &output.files, OutputFormat::Toon)?, @r#"
        {"command":"generate action","action":{"api_version":"takt.dev/v1alpha1","kind":"Action","name":"github-triage","capability":"@tuist/github#issues.list","description":"Project-local configured action scaffold"},"files":[{"label":"action","path":"actions/github-triage.json"}]}
        "#);
        Ok(())
    }

    #[test]
    fn workflow_text_output_matches_snapshot() -> Result<()> {
        let output = sample_workflow_output();
        insta::assert_snapshot!(render_scaffold_output(&output, &output.files, OutputFormat::Text)?, @r#"
        Wrote workflows/daily-triage.json
        "#);
        Ok(())
    }

    fn sample_action_output() -> ActionGenerateOutput {
        ActionGenerateOutput {
            command: "generate action",
            action: ActionDefinition::starter(
                "github-triage".into(),
                "@tuist/github#issues.list".into(),
            ),
            files: vec![WrittenFile {
                label: "action".into(),
                path: PathBuf::from("actions/github-triage.json"),
            }],
        }
    }

    fn sample_workflow_output() -> WorkflowGenerateOutput {
        WorkflowGenerateOutput {
            command: "generate workflow",
            workflow: WorkflowDefinition::starter("daily-triage".into(), "github-triage".into()),
            files: vec![WrittenFile {
                label: "workflow".into(),
                path: PathBuf::from("workflows/daily-triage.json"),
            }],
        }
    }
}
