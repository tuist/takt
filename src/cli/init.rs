use crate::cli::support::{write_scaffold_files, yaml_scaffold_file};
use crate::domain::PackageManifest;
use crate::scaffold::{package_bootstrap_files, package_project_root};
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
    pub(crate) fn run(self) -> Result<()> {
        let project_root = package_project_root(&self.output);
        let manifest = PackageManifest::starter(self.name.clone(), self.description);
        let mut files = vec![yaml_scaffold_file(&manifest, self.output, "package")?];
        files.extend(package_bootstrap_files(&project_root, &self.name));
        write_scaffold_files(&files, self.force)
    }
}
