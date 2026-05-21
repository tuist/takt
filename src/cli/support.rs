use crate::output::style;
use crate::scaffold::ScaffoldFile;
use color_eyre::eyre::{Result, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn write_yaml_file<T>(value: &T, output: &Path, force: bool, label: &str) -> Result<()>
where
    T: Serialize,
{
    let file = yaml_scaffold_file(value, output.to_path_buf(), label)?;
    write_scaffold_files(&[file], force)
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

pub(crate) fn write_scaffold_files(files: &[ScaffoldFile], force: bool) -> Result<()> {
    for file in files {
        if file.path.exists() && !force {
            bail!(
                "{} already exists at {}. Re-run with --force to overwrite.",
                file.label,
                file.path.display()
            );
        }
    }

    for file in files {
        if let Some(parent) = file.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&file.path, &file.contents)?;
        println!("{} {}", style::label("Wrote"), file.path.display());
    }

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
