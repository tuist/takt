use crate::cli::support::{CommandContext, OutputFormat, structured_json_string};
use crate::core;
use crate::output::style;
use clap::Args;
use color_eyre::eyre::Result;
use std::env;

#[derive(Debug, Args)]
pub(crate) struct PublishCommand {
    /// Publish under a specific dist-tag
    #[arg(short, long, env = "TAKT_PUBLISH_TAG")]
    tag: Option<String>,
    /// Access level to request from npm. Public scoped packages usually need `public` on first publish
    #[arg(long, env = "TAKT_PUBLISH_ACCESS", value_enum)]
    access: Option<core::PublishAccess>,
    /// Build the tarball and ask npm what it would publish without uploading it
    #[arg(short = 'n', long, env = "TAKT_PUBLISH_DRY_RUN")]
    dry_run: bool,
}

impl PublishCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let repo =
            core::discover_repository(context.package_dir.clone().unwrap_or(env::current_dir()?))?;
        let output = core::publish_package(&repo, self.tag, self.access, self.dry_run)?;
        print!("{}", render_output(&output, context.format)?);
        Ok(())
    }
}

fn render_output(output: &core::PublishOutput, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Text => {
            let published = format!("{}@{}", output.registry_package, output.version);
            let published = if let Some(access) = output.access {
                format!("{published} ({})", access.as_str())
            } else {
                published
            };
            Ok(format!(
                "{} {}\n{} {}\n",
                style::label("Packed"),
                output.tarball_path.display(),
                style::label(if output.published {
                    "Published"
                } else {
                    "Dry-run publish"
                }),
                published
            ))
        }
        OutputFormat::Json | OutputFormat::Toon => structured_json_string(output, format),
    }
}
