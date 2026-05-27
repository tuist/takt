use crate::cli::support::{CommandContext, OutputFormat, structured_json_string};
use crate::core;
use crate::output::style;
use clap::Args;
use color_eyre::eyre::Result;
use std::env;

#[derive(Debug, Args)]
pub(crate) struct InstallCommand {
    /// Reinstall every dependency even if the lockfile and store are fresh
    #[arg(short, long, env = "TAKT_INSTALL_FORCE")]
    force: bool,
}

impl InstallCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let repo =
            core::discover_repository(context.package_dir.clone().unwrap_or(env::current_dir()?))?;
        let output = core::install_dependencies(&repo, self.force)?;
        print!("{}", render_output(&output, context.format)?);
        Ok(())
    }
}

fn render_output(output: &core::InstallOutput, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Text => {
            let mut rendered = String::new();
            for dependency in &output.dependencies {
                rendered.push_str(&format!(
                    "{} {}@{}\n",
                    style::label("Installed"),
                    dependency.name,
                    dependency.version
                ));
                for skill in &dependency.projected_skills {
                    rendered.push_str(&format!(
                        "{} {}\n",
                        style::muted("Projected skill"),
                        skill.display()
                    ));
                }
            }
            rendered.push_str(&format!(
                "{} {}\n",
                style::label("Store"),
                output.store_root.display()
            ));
            rendered.push_str(&format!(
                "{} {}\n",
                style::label("Virtual store"),
                output.virtual_store_root.display()
            ));
            rendered.push_str(&format!(
                "{} {}\n",
                style::label("Wrote"),
                output.lockfile_path.display()
            ));
            Ok(rendered)
        }
        OutputFormat::Json | OutputFormat::Toon => structured_json_string(output, format),
    }
}
