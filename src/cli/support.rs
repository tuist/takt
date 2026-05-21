use crate::output::style;
use crate::scaffold::ScaffoldFile;
use clap::ValueEnum;
use color_eyre::eyre::{Result, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WrittenFile {
    pub label: String,
    pub path: PathBuf,
}

pub(crate) fn write_yaml_file<T>(
    value: &T,
    output: &Path,
    force: bool,
    label: &str,
) -> Result<WrittenFile>
where
    T: Serialize,
{
    let file = yaml_scaffold_file(value, output.to_path_buf(), label)?;
    let mut written = write_scaffold_files(&[file], force)?;
    Ok(written.remove(0))
}

pub(crate) fn yaml_scaffold_file<T>(value: &T, output: PathBuf, label: &str) -> Result<ScaffoldFile>
where
    T: Serialize,
{
    Ok(ScaffoldFile::new(
        output,
        label,
        serde_yaml::to_string(value)?,
    ))
}

pub(crate) fn write_scaffold_files(
    files: &[ScaffoldFile],
    force: bool,
) -> Result<Vec<WrittenFile>> {
    for file in files {
        if file.path.exists() && !force {
            bail!(
                "{} already exists at {}. Re-run with --force to overwrite.",
                file.label,
                file.path.display()
            );
        }
    }

    let mut written = Vec::with_capacity(files.len());

    for file in files {
        if let Some(parent) = file.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&file.path, &file.contents)?;
        written.push(WrittenFile {
            label: file.label.clone(),
            path: file.path.clone(),
        });
    }

    Ok(written)
}

pub(crate) fn print_written_files(files: &[WrittenFile]) {
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

pub(crate) fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_end_matches('-').to_string()
}
