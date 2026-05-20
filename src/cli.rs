use crate::domain::{
    ActionDefinition, CapabilityDefinition, PackageManifest, RuntimeProfile, WorkflowDefinition,
};
use crate::output::style;
use crate::output::table::TaktTable;
use crate::scaffold::{ScaffoldFile, package_bootstrap_files, package_project_root};
use clap::{Args, Parser, Subcommand, ValueEnum};
use color_eyre::eyre::{Result, bail};
use schemars::schema_for;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "takt",
    about = "Package-driven workflows for agent operations",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Concepts(command) => command.run(),
            Command::Package(command) => command.run(),
            Command::Action(command) => command.run(),
            Command::Workflow(command) => command.run(),
            Command::Schema(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the canonical Takt object model
    Concepts(ConceptsCommand),
    /// Create starter Takt package manifests
    Package(PackageCommand),
    /// Create starter Takt actions
    Action(ActionCommand),
    /// Create starter Takt workflows
    Workflow(WorkflowCommand),
    /// Emit machine-readable schemas for Takt domain objects
    Schema(SchemaCommand),
}

#[derive(Debug, Args)]
struct ConceptsCommand {
    /// Print the concepts in JSON instead of a table
    #[arg(long)]
    json: bool,
}

impl ConceptsCommand {
    fn run(self) -> Result<()> {
        let concepts = vec![
            ConceptRow::new(
                "Package",
                "Distributable unit published to a registry",
                "Registry",
                "Capabilities and runtime profiles",
            ),
            ConceptRow::new(
                "Capability",
                "Reusable interface exported by a package",
                "Package",
                "Runtime, handler, input schema, output schema",
            ),
            ConceptRow::new(
                "Action",
                "Project-local configured use of a capability",
                "Project",
                "Defaults, secret refs, labels, account selection",
            ),
            ConceptRow::new(
                "Workflow",
                "Ordered graph that composes actions",
                "Project",
                "Action steps plus dependencies",
            ),
            ConceptRow::new(
                "Run",
                "One execution of an action or workflow",
                "Runtime",
                "Logs, status, timings, provenance",
            ),
            ConceptRow::new(
                "Artifact",
                "Persisted output from a run",
                "Datastore",
                "Structured data or files",
            ),
        ];

        if self.json {
            println!("{}", serde_json::to_string_pretty(&concepts)?);
            return Ok(());
        }

        println!(
            "{} {}",
            style::title("Takt"),
            style::muted("package -> capability -> action -> workflow -> run -> artifact")
        );
        println!();

        let mut table = TaktTable::new(&["Concept", "Role", "Scope", "Carries"]);
        for concept in concepts {
            table.add_row([concept.name, concept.role, concept.scope, concept.carries]);
        }
        table.print()?;

        println!();
        println!(
            "{} {}",
            style::label("Runtime rule:"),
            "capabilities execute on named runtime profiles; workflows never point at images directly."
        );

        Ok(())
    }
}

#[derive(Debug, Args)]
struct PackageCommand {
    #[command(subcommand)]
    command: PackageSubcommand,
}

impl PackageCommand {
    fn run(self) -> Result<()> {
        match self.command {
            PackageSubcommand::Init(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum PackageSubcommand {
    /// Write a starter package manifest
    Init(PackageInitCommand),
}

#[derive(Debug, Args)]
struct PackageInitCommand {
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

impl PackageInitCommand {
    fn run(self) -> Result<()> {
        let project_root = package_project_root(&self.output);
        let manifest = PackageManifest::starter(self.name.clone(), self.description);
        let mut files = vec![yaml_scaffold_file(&manifest, self.output, "package")?];
        files.extend(package_bootstrap_files(&project_root, &self.name));
        write_scaffold_files(&files, self.force)
    }
}

#[derive(Debug, Args)]
struct ActionCommand {
    #[command(subcommand)]
    command: ActionSubcommand,
}

impl ActionCommand {
    fn run(self) -> Result<()> {
        match self.command {
            ActionSubcommand::Init(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum ActionSubcommand {
    /// Write a starter action manifest
    Init(ActionInitCommand),
}

#[derive(Debug, Args)]
struct ActionInitCommand {
    /// Action name
    name: String,
    /// Capability reference this action uses
    capability: String,
    /// Output path for the action manifest
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Overwrite an existing file
    #[arg(long)]
    force: bool,
}

impl ActionInitCommand {
    fn run(self) -> Result<()> {
        let output = self
            .output
            .unwrap_or_else(|| PathBuf::from(format!("actions/{}.yaml", slugify(&self.name))));
        let action = ActionDefinition::starter(self.name, self.capability);
        write_yaml_file(&action, &output, self.force, "action")
    }
}

#[derive(Debug, Args)]
struct WorkflowCommand {
    #[command(subcommand)]
    command: WorkflowSubcommand,
}

impl WorkflowCommand {
    fn run(self) -> Result<()> {
        match self.command {
            WorkflowSubcommand::Init(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum WorkflowSubcommand {
    /// Write a starter workflow manifest
    Init(WorkflowInitCommand),
}

#[derive(Debug, Args)]
struct WorkflowInitCommand {
    /// Workflow name
    name: String,
    /// Action reference used by the starter step
    #[arg(long, default_value = "example-action")]
    uses: String,
    /// Output path for the workflow manifest
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Overwrite an existing file
    #[arg(long)]
    force: bool,
}

impl WorkflowInitCommand {
    fn run(self) -> Result<()> {
        let output = self
            .output
            .unwrap_or_else(|| PathBuf::from(format!("workflows/{}.yaml", slugify(&self.name))));
        let workflow = WorkflowDefinition::starter(self.name, self.uses);
        write_yaml_file(&workflow, &output, self.force, "workflow")
    }
}

#[derive(Debug, Args)]
struct SchemaCommand {
    #[arg(value_enum, default_value_t = SchemaTarget::All)]
    target: SchemaTarget,
}

impl SchemaCommand {
    fn run(self) -> Result<()> {
        match self.target {
            SchemaTarget::All => {
                let bundle = SchemaBundle {
                    package: schema_for!(PackageManifest),
                    runtime: schema_for!(RuntimeProfile),
                    capability: schema_for!(CapabilityDefinition),
                    action: schema_for!(ActionDefinition),
                    workflow: schema_for!(WorkflowDefinition),
                };

                println!("{}", serde_json::to_string_pretty(&bundle)?);
            }
            SchemaTarget::Package => print_schema::<PackageManifest>()?,
            SchemaTarget::Runtime => print_schema::<RuntimeProfile>()?,
            SchemaTarget::Capability => print_schema::<CapabilityDefinition>()?,
            SchemaTarget::Action => print_schema::<ActionDefinition>()?,
            SchemaTarget::Workflow => print_schema::<WorkflowDefinition>()?,
        }

        Ok(())
    }
}

fn print_schema<T>() -> Result<()>
where
    T: schemars::JsonSchema + Serialize,
{
    println!("{}", serde_json::to_string_pretty(&schema_for!(T))?);
    Ok(())
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SchemaTarget {
    All,
    Package,
    Runtime,
    Capability,
    Action,
    Workflow,
}

#[derive(Debug, Serialize)]
struct ConceptRow {
    name: &'static str,
    role: &'static str,
    scope: &'static str,
    carries: &'static str,
}

impl ConceptRow {
    const fn new(
        name: &'static str,
        role: &'static str,
        scope: &'static str,
        carries: &'static str,
    ) -> Self {
        Self {
            name,
            role,
            scope,
            carries,
        }
    }
}

#[derive(Debug, Serialize)]
struct SchemaBundle {
    package: schemars::schema::RootSchema,
    runtime: schemars::schema::RootSchema,
    capability: schemars::schema::RootSchema,
    action: schemars::schema::RootSchema,
    workflow: schemars::schema::RootSchema,
}

fn write_yaml_file<T>(value: &T, output: &Path, force: bool, label: &str) -> Result<()>
where
    T: Serialize,
{
    let file = yaml_scaffold_file(value, output.to_path_buf(), label)?;
    write_scaffold_files(&[file], force)
}

fn yaml_scaffold_file<T>(value: &T, output: PathBuf, label: &str) -> Result<ScaffoldFile>
where
    T: Serialize,
{
    Ok(ScaffoldFile::new(
        output,
        label,
        serde_yaml::to_string(value)?,
    ))
}

fn write_scaffold_files(files: &[ScaffoldFile], force: bool) -> Result<()> {
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

fn slugify(input: &str) -> String {
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
