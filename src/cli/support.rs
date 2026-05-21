use crate::output::style;
use clap::ValueEnum;
use color_eyre::eyre::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandContext {
    pub format: OutputFormat,
    pub repo_dir: Option<PathBuf>,
}

pub(crate) fn print_written_files(files: &[crate::core::WrittenFile]) {
    for file in files {
        println!("{} {}", style::label("Wrote"), file.path.display());
    }
}

pub(crate) fn print_data<T>(value: &T, format: OutputFormat) -> Result<()>
where
    T: Serialize,
{
    match format {
        OutputFormat::Text => print!("{}", serde_yaml::to_string(value)?),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value)?),
    }

    Ok(())
}

pub(crate) fn print_json<T>(value: &T) -> Result<()>
where
    T: Serialize,
{
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
