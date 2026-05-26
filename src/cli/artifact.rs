use crate::cli::support::{CommandContext, print_data};
use crate::core;
use crate::datastore::ArtifactRecord;
use crate::query::parse_key_value;
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, bail};
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub(crate) struct ArtifactCommand {
    #[command(subcommand)]
    command: ArtifactSubcommand,
}

impl ArtifactCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let repo =
            core::discover_repository(context.package_dir.clone().unwrap_or(env::current_dir()?))?;

        match self.command {
            ArtifactSubcommand::List(command) => {
                let mut tags = BTreeMap::new();
                for raw in &command.tag {
                    let (k, v) = parse_key_value(raw)?;
                    tags.insert(k, v);
                }
                let input = core::ArtifactListInput {
                    run: command.run,
                    name: command.name,
                    capability: command.capability,
                    tags,
                    since: command.since,
                    limit: command.limit,
                    predicates: command.r#where,
                };
                print_data(
                    &core::artifact_list_envelope(&repo, &input)?,
                    context.format,
                )
            }
            ArtifactSubcommand::Get(command) => match core::get_artifact(&repo, &command.id)? {
                Some(artifact) => print_data(
                    &ArtifactGetOutput {
                        command: "artifact get",
                        artifact,
                    },
                    context.format,
                ),
                None => bail!("artifact '{}' was not found in the datastore", command.id),
            },
        }
    }
}

#[derive(Debug, Serialize, JsonSchema)]
struct ArtifactGetOutput {
    command: &'static str,
    artifact: ArtifactRecord,
}

#[derive(Debug, Subcommand)]
enum ArtifactSubcommand {
    /// List artifacts persisted in the datastore
    List(ArtifactListCommand),
    /// Get a single artifact record by id
    Get(ArtifactGetCommand),
}

#[derive(Debug, Args)]
struct ArtifactListCommand {
    /// Filter by run id
    #[arg(long, value_name = "RUN_ID")]
    run: Option<String>,
    /// Filter by artifact name
    #[arg(long, value_name = "NAME")]
    name: Option<String>,
    /// Filter by producing capability name (matches producer_kind=capability)
    #[arg(long, value_name = "CAPABILITY")]
    capability: Option<String>,
    /// Require a tag value (repeatable): --tag env=prod --tag role=writer
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    tag: Vec<String>,
    /// Only include artifacts created within the given duration (e.g. 30s, 5m, 2h, 7d)
    #[arg(long, value_name = "DURATION")]
    since: Option<String>,
    /// Maximum number of artifacts to return (newest first)
    #[arg(long, value_name = "N")]
    limit: Option<usize>,
    /// Equality predicate over a record path (repeatable): --where tags.env=prod
    #[arg(long = "where", value_name = "PATH=VALUE")]
    r#where: Vec<String>,
}

#[derive(Debug, Args)]
struct ArtifactGetCommand {
    /// Artifact id
    id: String,
}
