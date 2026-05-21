use crate::cli::support::{CommandContext, OutputFormat, print_json, print_written_files};
use crate::core;
use clap::Args;
use color_eyre::eyre::Result;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(crate) struct InitCommand {
    /// Package name to write into the manifest
    name: String,
    /// Optional package description
    #[arg(long)]
    description: Option<String>,
    /// Output path for the package manifest
    #[arg(short, long, default_value = "package.yaml", value_name = "PATH")]
    output: PathBuf,
    /// Overwrite an existing file
    #[arg(long)]
    force: bool,
}

impl InitCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let output = core::init_package(self.name, self.description, self.output, self.force)?;

        match context.format {
            OutputFormat::Text => {
                print_written_files(&output.files);
                Ok(())
            }
            OutputFormat::Json => print_json(&output),
        }
    }
}
