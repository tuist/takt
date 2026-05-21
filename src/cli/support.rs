use crate::output::style;
use clap::ValueEnum;
use color_eyre::eyre::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Text,
    Json,
    Toon,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandContext {
    pub format: OutputFormat,
    pub package_dir: Option<PathBuf>,
}

pub(crate) fn written_files_summary(files: &[crate::core::WrittenFile]) -> String {
    let mut summary = String::new();

    for file in files {
        summary.push_str(&format!(
            "{} {}\n",
            style::label("Wrote"),
            file.path.display()
        ));
    }

    summary
}

pub(crate) fn data_string<T>(value: &T, format: OutputFormat) -> Result<String>
where
    T: Serialize,
{
    Ok(match format {
        OutputFormat::Text | OutputFormat::Json => {
            format!("{}\n", serde_json::to_string_pretty(value)?)
        }
        OutputFormat::Toon => format!("{}\n", serde_json::to_string(value)?),
    })
}

pub(crate) fn print_data<T>(value: &T, format: OutputFormat) -> Result<()>
where
    T: Serialize,
{
    print!("{}", data_string(value, format)?);
    Ok(())
}

pub(crate) fn structured_json_string<T>(value: &T, format: OutputFormat) -> Result<String>
where
    T: Serialize,
{
    Ok(match format {
        OutputFormat::Toon => format!("{}\n", serde_json::to_string(value)?),
        OutputFormat::Text | OutputFormat::Json => {
            format!("{}\n", serde_json::to_string_pretty(value)?)
        }
    })
}

pub(crate) fn print_structured_json<T>(value: &T, format: OutputFormat) -> Result<()>
where
    T: Serialize,
{
    print!("{}", structured_json_string(value, format)?);
    Ok(())
}
