use crate::cli::support::{
    CommandContext, OutputFormat, print_structured_json, print_written_files,
};
use crate::core;
use crate::scaffold::CodingAgent;
use clap::Args;
use color_eyre::eyre::Result;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(crate) struct InitCommand {
    /// Package name to write into the manifest
    name: String,
    /// Optional package description
    #[arg(short, long, env = "TAKT_INIT_DESCRIPTION")]
    description: Option<String>,
    /// Output path for the package manifest
    #[arg(
        short,
        long,
        env = "TAKT_INIT_OUTPUT",
        default_value = "package.yaml",
        value_name = "PATH"
    )]
    output: PathBuf,
    /// Coding-agent bootstrap to write into the package
    #[arg(short = 'a', long, env = "TAKT_INIT_CODING_AGENT", value_enum, default_value_t = CodingAgent::Codex)]
    coding_agent: CodingAgent,
    /// Overwrite an existing file
    #[arg(short, long, env = "TAKT_INIT_FORCE")]
    force: bool,
}

impl InitCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let output = core::init_package(
            self.name,
            self.description,
            self.output,
            self.force,
            self.coding_agent,
        )?;

        match context.format {
            OutputFormat::Text => {
                print_written_files(&output.files);
                Ok(())
            }
            OutputFormat::Json | OutputFormat::Toon => {
                print_structured_json(&output, context.format)
            }
        }
    }
}
