use crate::domain::{
    API_VERSION, ActionDefinition, CapabilityDefinition, HandlerDefinition, LockedPackage,
    PackageJsonManifest, PackageManifest, TaktLockfile, WorkflowDefinition,
};
use crate::scaffold::{CodingAgent, ScaffoldFile, package_bootstrap_files, package_project_root};
use base64::Engine;
use clap::ValueEnum;
use color_eyre::eyre::{Result, bail, eyre};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use reqwest::blocking::Client;
use schemars::schema_for;
use schemars::{JsonSchema, Schema};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::{Archive, Builder, Header};

pub const CONCEPT_CHAIN: &str = "package -> capability -> action -> workflow -> run -> artifact";
pub const EXECUTION_RULE: &str =
    "packages pin an exact Node version; workflows never point at scripts directly.";
pub const ROOT_MANIFEST_FILENAME: &str = "takt.json";
pub const PACKAGE_JSON_FILENAME: &str = "package.json";
pub const LOCKFILE_FILENAME: &str = "takt.lock.json";
pub const MANIFEST_EXTENSION: &str = "json";
pub const STORE_DIRECTORY_ENV: &str = "TAKT_STORE_DIR";
pub const STORE_VERSION: &str = "v1";
pub const STORE_FILES_SUBDIR: &str = "files";
pub const STORE_INDEX_SUBDIR: &str = "index";
pub const STORE_VIEWS_SUBDIR: &str = "virtual-store";
pub const CACHE_DIRECTORY_NAME: &str = "takt";

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ConceptsOutput {
    pub chain: &'static str,
    pub execution_rule: &'static str,
    pub concepts: Vec<ConceptRow>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ConceptRow {
    pub name: &'static str,
    pub role: &'static str,
    pub scope: &'static str,
    pub carries: &'static str,
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

pub fn concepts() -> ConceptsOutput {
    ConceptsOutput {
        chain: CONCEPT_CHAIN,
        execution_rule: EXECUTION_RULE,
        concepts: vec![
            ConceptRow::new(
                "Package",
                "Distributable unit published to a registry",
                "Registry",
                "Node version and capabilities",
            ),
            ConceptRow::new(
                "Capability",
                "Reusable interface exported by a package",
                "Package",
                "Handler, input schema, output schema",
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
                "Executor",
                "Logs, status, timings, provenance",
            ),
            ConceptRow::new(
                "Artifact",
                "Persisted output from a run",
                "Datastore",
                "Structured data or files",
            ),
        ],
    }
}

#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaTarget {
    All,
    Package,
    Capability,
    Action,
    Workflow,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PublishAccess {
    Public,
    Restricted,
}

impl PublishAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Restricted => "restricted",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SchemaBundle {
    pub package: Schema,
    pub capability: Schema,
    pub action: Schema,
    pub workflow: Schema,
}

pub fn schema_bundle() -> SchemaBundle {
    SchemaBundle {
        package: schema_for!(PackageManifest),
        capability: schema_for!(CapabilityDefinition),
        action: schema_for!(ActionDefinition),
        workflow: schema_for!(WorkflowDefinition),
    }
}

pub fn schema_for_target(target: SchemaTarget) -> Value {
    match target {
        SchemaTarget::All => serde_json::to_value(schema_bundle()).expect("schema bundle is valid"),
        SchemaTarget::Package => {
            serde_json::to_value(schema_for!(PackageManifest)).expect("package schema is valid")
        }
        SchemaTarget::Capability => serde_json::to_value(schema_for!(CapabilityDefinition))
            .expect("capability schema is valid"),
        SchemaTarget::Action => {
            serde_json::to_value(schema_for!(ActionDefinition)).expect("action schema is valid")
        }
        SchemaTarget::Workflow => {
            serde_json::to_value(schema_for!(WorkflowDefinition)).expect("workflow schema is valid")
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WrittenFile {
    pub label: String,
    pub path: PathBuf,
}

pub fn write_json_file<T>(value: &T, output: &Path, force: bool, label: &str) -> Result<WrittenFile>
where
    T: Serialize,
{
    let file = json_scaffold_file(value, output.to_path_buf(), label)?;
    let mut written = write_scaffold_files(&[file], force)?;
    Ok(written.remove(0))
}

pub fn json_scaffold_file<T>(value: &T, output: PathBuf, label: &str) -> Result<ScaffoldFile>
where
    T: Serialize,
{
    Ok(ScaffoldFile::new(
        output,
        label,
        format!("{}\n", serde_json::to_string_pretty(value)?),
    ))
}

pub fn write_scaffold_files(files: &[ScaffoldFile], force: bool) -> Result<Vec<WrittenFile>> {
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
        if let Some(parent) = file.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        fs::write(&file.path, &file.contents)?;
        written.push(WrittenFile {
            label: file.label.clone(),
            path: file.path.clone(),
        });
    }

    Ok(written)
}

pub fn slugify(input: &str) -> String {
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

#[derive(Debug, Serialize, JsonSchema)]
pub struct InitOutput {
    pub command: &'static str,
    pub coding_agent: CodingAgent,
    pub package: PackageManifest,
    pub package_json: PackageJsonManifest,
    pub files: Vec<WrittenFile>,
}

pub fn init_package(
    name: String,
    description: Option<String>,
    output: PathBuf,
    force: bool,
    coding_agent: CodingAgent,
) -> Result<InitOutput> {
    let project_root = package_project_root(&output);
    let manifest = PackageManifest::starter(name.clone(), description);
    let package_json = PackageJsonManifest::from_package_manifest(&manifest);
    let mut files = vec![
        json_scaffold_file(&manifest, output, "package")?,
        json_scaffold_file(
            &package_json,
            project_root.join(PACKAGE_JSON_FILENAME),
            "npm package",
        )?,
    ];
    files.extend(package_bootstrap_files(&project_root, &name, coding_agent));
    let written = write_scaffold_files(&files, force)?;

    Ok(InitOutput {
        command: "init",
        coding_agent,
        package: manifest,
        package_json,
        files: written,
    })
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ActionGenerateOutput {
    pub command: &'static str,
    pub action: ActionDefinition,
    pub files: Vec<WrittenFile>,
}

pub fn generate_action(
    name: String,
    capability: String,
    output: Option<PathBuf>,
    force: bool,
) -> Result<ActionGenerateOutput> {
    let output = output.unwrap_or_else(|| {
        PathBuf::from(format!("actions/{}.{}", slugify(&name), MANIFEST_EXTENSION))
    });
    let action = ActionDefinition::starter(name, capability);
    let written = write_json_file(&action, &output, force, "action")?;

    Ok(ActionGenerateOutput {
        command: "generate action",
        action,
        files: vec![written],
    })
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct WorkflowGenerateOutput {
    pub command: &'static str,
    pub workflow: WorkflowDefinition,
    pub files: Vec<WrittenFile>,
}

pub fn generate_workflow(
    name: String,
    uses: String,
    output: Option<PathBuf>,
    force: bool,
) -> Result<WorkflowGenerateOutput> {
    let output = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "workflows/{}.{}",
            slugify(&name),
            MANIFEST_EXTENSION
        ))
    });
    let workflow = WorkflowDefinition::starter(name, uses);
    let written = write_json_file(&workflow, &output, force, "workflow")?;

    Ok(WorkflowGenerateOutput {
        command: "generate workflow",
        workflow,
        files: vec![written],
    })
}

pub fn install_dependencies(repo: &Repository, force: bool) -> Result<InstallOutput> {
    let registry = load_registry_config(repo)?;
    let mut lockfile = TaktLockfile::empty();
    let mut installed = BTreeMap::new();
    let mut pending = repo
        .package_json
        .dependencies
        .iter()
        .map(|(name, specifier)| (name.clone(), specifier.clone()))
        .collect::<Vec<_>>();

    while let Some((name, specifier)) = pending.pop() {
        if let Some(existing) = lockfile.packages.get(&name) {
            if dependency_specifier_matches_version(&specifier, &existing.version) {
                continue;
            }
            bail!(
                "dependency graph requires conflicting versions for '{}': '{}' and '{}'",
                name,
                existing.specifier,
                specifier
            );
        }

        let (version, resolved, integrity, store_path) =
            if let Some(existing) = repo.lockfile.packages.get(&name) {
                if !force && existing.specifier == specifier {
                    (
                        existing.version.clone(),
                        existing.resolved.clone(),
                        existing.integrity.clone(),
                        ensure_cached_package_view(
                            repo,
                            &registry,
                            &name,
                            &specifier,
                            &existing.version,
                            &existing.resolved,
                            &existing.integrity,
                            false,
                        )?,
                    )
                } else {
                    resolve_and_materialize_dependency(repo, &registry, &name, &specifier, force)?
                }
            } else {
                resolve_and_materialize_dependency(repo, &registry, &name, &specifier, force)?
            };

        lockfile.packages.insert(
            name.clone(),
            LockedPackage {
                specifier: specifier.clone(),
                version: version.clone(),
                resolved: resolved.clone(),
                integrity: integrity.clone(),
            },
        );
        for dependency in load_installed_package_json(&store_path)?.dependencies {
            pending.push(dependency);
        }
        installed.insert(
            name.clone(),
            InstalledDependency {
                name: name.clone(),
                specifier: specifier.clone(),
                version,
                resolved,
                integrity,
                store_path,
                projected_skills: Vec::new(),
            },
        );
    }

    write_json_value(&lockfile, &repo.lockfile_path)?;
    let mut installed = installed.into_values().collect::<Vec<_>>();
    installed.sort_by(|left, right| left.name.cmp(&right.name));
    let projected = project_dependency_skills(repo, &installed)?;
    for dependency in &mut installed {
        dependency.projected_skills = projected.get(&dependency.name).cloned().unwrap_or_default();
    }

    Ok(InstallOutput {
        command: "install",
        lockfile_path: repo.lockfile_path.clone(),
        store_root: repo.store_root.clone(),
        virtual_store_root: virtual_store_root(&repo.cache_root),
        dependencies: installed,
    })
}

pub fn publish_package(
    repo: &Repository,
    tag: Option<String>,
    access: Option<PublishAccess>,
    dry_run: bool,
) -> Result<PublishOutput> {
    if !repo.package_json_present {
        bail!(
            "cannot publish without {PACKAGE_JSON_FILENAME}; run `takt init` again or add it manually"
        );
    }

    let tarball_path = build_publish_tarball(repo)?;
    let mut command = Command::new("npm");
    command.args(npm_publish_arguments(
        &tarball_path,
        tag.as_deref(),
        access,
        dry_run,
    ));
    command.current_dir(&repo.root);

    let status = command
        .status()
        .map_err(|error| eyre!("failed to spawn npm publish: {error}"))?;
    if !status.success() {
        bail!("npm publish failed with status {status}");
    }

    Ok(PublishOutput {
        command: "publish",
        registry_package: repo.package_json.name.clone(),
        version: repo.package_json.version.clone(),
        tarball_path,
        access,
        published: !dry_run,
    })
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
    pub package_path: PathBuf,
    pub package: PackageManifest,
    pub package_json_path: PathBuf,
    pub package_json: PackageJsonManifest,
    pub package_json_present: bool,
    pub lockfile_path: PathBuf,
    pub lockfile: TaktLockfile,
    pub store_root: PathBuf,
    pub cache_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InstalledDependency {
    pub name: String,
    pub specifier: String,
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub store_path: PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projected_skills: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InstallOutput {
    pub command: &'static str,
    pub lockfile_path: PathBuf,
    pub store_root: PathBuf,
    pub virtual_store_root: PathBuf,
    pub dependencies: Vec<InstalledDependency>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PublishOutput {
    pub command: &'static str,
    pub registry_package: String,
    pub version: String,
    pub tarball_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<PublishAccess>,
    pub published: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct NpmPackageDocument {
    #[serde(rename = "dist-tags", default)]
    dist_tags: BTreeMap<String, String>,
    #[serde(default)]
    versions: BTreeMap<String, NpmVersionDocument>,
}

#[derive(Debug, Clone, Deserialize)]
struct NpmVersionDocument {
    dist: NpmDistDocument,
}

#[derive(Debug, Clone, Deserialize)]
struct NpmDistDocument {
    tarball: String,
    integrity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFile {
    hex_hash: String,
    store_path: PathBuf,
    executable: bool,
    size: u64,
}

type PackageIndex = BTreeMap<String, StoredFile>;

#[derive(Debug, Clone)]
struct RegistryConfig {
    default_registry: String,
    scoped_registries: BTreeMap<String, String>,
    auth_tokens: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ActionDocument {
    pub path: PathBuf,
    pub action: ActionDefinition,
}

#[derive(Debug, Clone)]
pub struct WorkflowDocument {
    pub path: PathBuf,
    pub workflow: WorkflowDefinition,
}

pub fn discover_repository(start: impl AsRef<Path>) -> Result<Repository> {
    let root = find_repo_root(start.as_ref())?;
    let package_path = root.join(ROOT_MANIFEST_FILENAME);
    let package = read_json_file(&package_path)?;
    let package_json_path = root.join(PACKAGE_JSON_FILENAME);
    let package_json_present = package_json_path.exists();
    let package_json = if package_json_present {
        read_json_file(&package_json_path)?
    } else {
        PackageJsonManifest::from_package_manifest(&package)
    };
    let lockfile_path = root.join(LOCKFILE_FILENAME);
    let lockfile = if lockfile_path.exists() {
        read_json_file(&lockfile_path)?
    } else {
        TaktLockfile::empty()
    };
    let store_root = resolve_store_root(&root);
    let cache_root = resolve_cache_root(&root);
    Ok(Repository {
        root,
        package_path,
        package,
        package_json_path,
        package_json,
        package_json_present,
        lockfile_path,
        lockfile,
        store_root,
        cache_root,
    })
}

pub fn load_action(repo: &Repository, selector: &str) -> Result<ActionDocument> {
    let path = resolve_manifest_path(&repo.root, "actions", selector, "action")?;
    let action = read_json_file(&path)?;
    Ok(ActionDocument { path, action })
}

pub fn load_workflow(repo: &Repository, selector: &str) -> Result<WorkflowDocument> {
    let path = resolve_manifest_path(&repo.root, "workflows", selector, "workflow")?;
    let workflow = read_json_file(&path)?;
    Ok(WorkflowDocument { path, workflow })
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationReport {
    pub kind: String,
    pub subject: String,
    pub path: PathBuf,
    pub checks: Vec<ValidationCheck>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationSummary {
    pub reports: Vec<ValidationReport>,
    pub passed: bool,
}

pub fn validate_package(repo: &Repository) -> ValidationReport {
    let package = &repo.package;
    let mut checks = vec![
        expect_equal(
            "API version",
            &package.api_version,
            API_VERSION,
            "package manifest api_version",
        ),
        simple_check(
            "Package name",
            !package.name.trim().is_empty(),
            "package name must not be empty",
        ),
        simple_check(
            "Package version",
            !package.version.trim().is_empty(),
            "package version must not be empty",
        ),
        simple_check(
            "Node version",
            !package.node.trim().is_empty(),
            "package node must not be empty",
        ),
    ];

    checks.push(simple_check(
        "package.json exists or can be synthesized",
        true,
        if repo.package_json_present {
            ""
        } else {
            "package.json is missing; dependency installation and npm publishing are disabled until it is added"
        },
    ));
    checks.push(expect_equal(
        "package.json name",
        &repo.package_json.name,
        &package.name,
        "package.json name",
    ));
    checks.push(expect_equal(
        "package.json version",
        &repo.package_json.version,
        &package.version,
        "package.json version",
    ));

    for (name, capability) in &package.capabilities {
        checks.push(simple_check(
            format!("Capability '{name}' handler entrypoint is present"),
            !capability.handler.entrypoint.trim().is_empty(),
            format!("capability '{name}' handler entrypoint must not be empty"),
        ));
        checks.push(simple_check(
            format!("Capability '{name}' input schema is present"),
            !capability.input.path.trim().is_empty(),
            format!("capability '{name}' input schema path must not be empty"),
        ));
        checks.push(simple_check(
            format!("Capability '{name}' output schema is present"),
            !capability.output.path.trim().is_empty(),
            format!("capability '{name}' output schema path must not be empty"),
        ));
    }

    validation_report(
        "package",
        package.name.clone(),
        repo.package_path.clone(),
        checks,
    )
}

pub fn validate_action_document(repo: &Repository, document: &ActionDocument) -> ValidationReport {
    let action = &document.action;
    let mut checks = Vec::new();

    checks.push(expect_equal(
        "API version",
        &action.api_version,
        API_VERSION,
        "action api_version",
    ));
    checks.push(expect_equal("Kind", &action.kind, "Action", "action kind"));
    checks.push(simple_check(
        "Action name",
        !action.name.trim().is_empty(),
        "action name must not be empty",
    ));

    let capability_resolution = resolve_capability_reference(repo, &action.capability);
    checks.push(match &capability_resolution {
        CapabilityResolution::Local { reference, .. } => simple_check(
            format!("Local capability '{}' exists in package", reference),
            true,
            "",
        ),
        CapabilityResolution::InstalledExternal {
            package,
            capability,
            version,
            ..
        } => simple_check(
            format!(
                "External capability '{}#{}' is installed from {}",
                package, capability, version
            ),
            true,
            "",
        ),
        CapabilityResolution::MissingDependency { package, .. } => simple_check(
            format!("Dependency '{}' is declared in package.json", package),
            false,
            format!("add '{package}' to package.json dependencies"),
        ),
        CapabilityResolution::UninstalledDependency {
            package, specifier, ..
        } => simple_check(
            format!("Dependency '{}' is installed", package),
            false,
            format!(
                "dependency '{}' is declared as '{}' but not installed; run `takt install`",
                package, specifier
            ),
        ),
        CapabilityResolution::MissingExternalCapability {
            package,
            version,
            capability,
            ..
        } => simple_check(
            format!(
                "Installed dependency '{}' exports '{}'",
                package, capability
            ),
            false,
            format!(
                "installed package '{}' at version '{}' does not export capability '{}'",
                package, version, capability
            ),
        ),
        CapabilityResolution::Invalid { reference, reason } => simple_check(
            format!("Capability reference '{}' is valid", reference),
            false,
            reason.clone(),
        ),
        CapabilityResolution::MissingLocal { reference } => simple_check(
            format!("Local capability '{}' exists in package", reference),
            false,
            format!("capability '{reference}' is not defined in {ROOT_MANIFEST_FILENAME}"),
        ),
    });

    validation_report("action", action.name.clone(), document.path.clone(), checks)
}

pub fn validate_workflow_document(
    repo: &Repository,
    document: &WorkflowDocument,
) -> ValidationReport {
    let workflow = &document.workflow;
    let mut checks = vec![
        expect_equal(
            "API version",
            &workflow.api_version,
            API_VERSION,
            "workflow api_version",
        ),
        expect_equal("Kind", &workflow.kind, "Workflow", "workflow kind"),
        simple_check(
            "Workflow name",
            !workflow.name.trim().is_empty(),
            "workflow name must not be empty",
        ),
        simple_check(
            "Workflow steps",
            !workflow.steps.is_empty(),
            "workflow must declare at least one step",
        ),
    ];

    let mut step_names = BTreeSet::new();
    let mut duplicate_names = Vec::new();
    for step in &workflow.steps {
        if !step_names.insert(step.name.clone()) {
            duplicate_names.push(step.name.clone());
        }
    }
    checks.push(simple_check(
        "Step names are unique",
        duplicate_names.is_empty(),
        format!("duplicate step names: {}", duplicate_names.join(", ")),
    ));

    for step in &workflow.steps {
        checks.push(match load_action(repo, &step.uses) {
            Ok(_) => simple_check(
                format!("Step '{}' references an existing action", step.name),
                true,
                "",
            ),
            Err(error) => simple_check(
                format!("Step '{}' references an existing action", step.name),
                false,
                format!("{} ({error})", step.uses),
            ),
        });

        for dependency in &step.needs {
            checks.push(simple_check(
                format!("Step '{}' dependency '{}' exists", step.name, dependency),
                step_names.contains(dependency),
                format!(
                    "step '{}' depends on unknown step '{}'",
                    step.name, dependency
                ),
            ));
        }
    }

    checks.push(simple_check(
        "Step dependency graph is acyclic",
        !workflow_has_cycle(workflow),
        "workflow contains a cycle in step dependencies",
    ));

    validation_report(
        "workflow",
        workflow.name.clone(),
        document.path.clone(),
        checks,
    )
}

pub fn validate_all(repo: &Repository) -> Result<ValidationSummary> {
    let mut reports = vec![validate_package(repo)];

    for path in list_manifest_files(repo.root.join("actions"))? {
        let action = ActionDocument {
            action: read_json_file(&path)?,
            path,
        };
        reports.push(validate_action_document(repo, &action));
    }

    for path in list_manifest_files(repo.root.join("workflows"))? {
        let workflow = WorkflowDocument {
            workflow: read_json_file(&path)?,
            path,
        };
        reports.push(validate_workflow_document(repo, &workflow));
    }

    let passed = reports.iter().all(|report| report.passed);
    Ok(ValidationSummary { reports, passed })
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Planned,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RunMode {
    PlanOnly,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum CapabilityResolution {
    Local {
        reference: String,
        package: String,
        capability: String,
        node: String,
        handler: HandlerDefinition,
    },
    InstalledExternal {
        reference: String,
        package: String,
        specifier: String,
        version: String,
        capability: String,
        node: String,
        handler: HandlerDefinition,
        manifest_path: PathBuf,
        store_path: PathBuf,
    },
    MissingDependency {
        reference: String,
        package: String,
    },
    UninstalledDependency {
        reference: String,
        package: String,
        specifier: String,
    },
    MissingExternalCapability {
        reference: String,
        package: String,
        version: String,
        capability: String,
        manifest_path: PathBuf,
    },
    MissingLocal {
        reference: String,
    },
    Invalid {
        reference: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ActionRunTarget {
    pub name: String,
    pub path: PathBuf,
    pub capability: String,
    pub resolution: CapabilityResolution,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ActionRunRecord {
    pub id: String,
    pub status: RunStatus,
    pub mode: RunMode,
    pub planned_at_unix_ms: u64,
    pub repo_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_path: Option<PathBuf>,
    pub message: String,
    pub inputs: BTreeMap<String, Value>,
    pub validation: ValidationReport,
    pub action: ActionRunTarget,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ActionRunOutput {
    pub command: &'static str,
    pub run: ActionRunRecord,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkflowStepRunTarget {
    pub name: String,
    pub action: String,
    pub action_path: PathBuf,
    pub capability: String,
    pub resolution: CapabilityResolution,
    pub needs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkflowRunTarget {
    pub name: String,
    pub path: PathBuf,
    pub steps: Vec<WorkflowStepRunTarget>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkflowRunRecord {
    pub id: String,
    pub status: RunStatus,
    pub mode: RunMode,
    pub planned_at_unix_ms: u64,
    pub repo_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_path: Option<PathBuf>,
    pub message: String,
    pub inputs: BTreeMap<String, Value>,
    pub validation: ValidationReport,
    pub workflow: WorkflowRunTarget,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkflowRunOutput {
    pub command: &'static str,
    pub run: WorkflowRunRecord,
}

pub fn parse_input_assignments(assignments: &[String]) -> Result<BTreeMap<String, Value>> {
    let mut inputs = BTreeMap::new();

    for assignment in assignments {
        let (key, raw_value) = assignment
            .split_once('=')
            .ok_or_else(|| eyre!("invalid input '{assignment}', expected key=value"))?;
        let value =
            serde_json::from_str(raw_value).unwrap_or_else(|_| Value::String(raw_value.into()));
        inputs.insert(key.to_string(), value);
    }

    Ok(inputs)
}

pub fn plan_action_run(
    repo: &Repository,
    selector: &str,
    inputs: BTreeMap<String, Value>,
    persist: bool,
) -> Result<ActionRunOutput> {
    let document = load_action(repo, selector)?;
    let validation = validate_action_document(repo, &document);
    if !validation.passed {
        bail!("action '{}' failed validation", document.action.name);
    }

    let resolution = resolve_capability_reference(repo, &document.action.capability);
    if let Some(error) = capability_resolution_error(&resolution) {
        bail!(
            "cannot plan run for {}: {error}",
            document.action.capability
        );
    }

    let planned_at_unix_ms = now_unix_ms()?;
    let id = format!("run-{planned_at_unix_ms}");
    let mut record = ActionRunRecord {
        id: id.clone(),
        status: RunStatus::Planned,
        mode: RunMode::PlanOnly,
        planned_at_unix_ms,
        repo_root: repo.root.clone(),
        state_path: None,
        message: "Execution is not implemented yet; Takt persisted a planned run only.".into(),
        inputs,
        validation,
        action: ActionRunTarget {
            name: document.action.name,
            path: document.path,
            capability: document.action.capability,
            resolution,
        },
    };

    if persist {
        let state_path = persist_run_record(&repo.root, &id, &record)?;
        record.state_path = Some(state_path);
    }

    Ok(ActionRunOutput {
        command: "run action",
        run: record,
    })
}

pub fn plan_workflow_run(
    repo: &Repository,
    selector: &str,
    inputs: BTreeMap<String, Value>,
    persist: bool,
) -> Result<WorkflowRunOutput> {
    let document = load_workflow(repo, selector)?;
    let validation = validate_workflow_document(repo, &document);
    if !validation.passed {
        bail!("workflow '{}' failed validation", document.workflow.name);
    }

    let mut steps = Vec::with_capacity(document.workflow.steps.len());
    for step in &document.workflow.steps {
        let action_document = load_action(repo, &step.uses)?;
        let action_validation = validate_action_document(repo, &action_document);
        if !action_validation.passed {
            bail!(
                "workflow step '{}' references action '{}' that failed validation",
                step.name,
                action_document.action.name
            );
        }
        let resolution = resolve_capability_reference(repo, &action_document.action.capability);
        if let Some(error) = capability_resolution_error(&resolution) {
            bail!(
                "workflow step '{}' references an invalid capability '{}': {error}",
                step.name,
                action_document.action.capability
            );
        }

        steps.push(WorkflowStepRunTarget {
            name: step.name.clone(),
            action: action_document.action.name.clone(),
            action_path: action_document.path.clone(),
            capability: action_document.action.capability.clone(),
            resolution,
            needs: step.needs.clone(),
        });
    }

    let planned_at_unix_ms = now_unix_ms()?;
    let id = format!("run-{planned_at_unix_ms}");
    let mut record = WorkflowRunRecord {
        id: id.clone(),
        status: RunStatus::Planned,
        mode: RunMode::PlanOnly,
        planned_at_unix_ms,
        repo_root: repo.root.clone(),
        state_path: None,
        message: "Execution is not implemented yet; Takt persisted a planned run only.".into(),
        inputs,
        validation,
        workflow: WorkflowRunTarget {
            name: document.workflow.name,
            path: document.path,
            steps,
        },
    };

    if persist {
        let state_path = persist_run_record(&repo.root, &id, &record)?;
        record.state_path = Some(state_path);
    }

    Ok(WorkflowRunOutput {
        command: "run workflow",
        run: record,
    })
}

fn find_repo_root(start: &Path) -> Result<PathBuf> {
    let start = if start.is_file() {
        start
            .parent()
            .ok_or_else(|| eyre!("unable to resolve package root from file path"))?
    } else {
        start
    };

    for candidate in start.ancestors() {
        if candidate.join(ROOT_MANIFEST_FILENAME).exists() {
            return Ok(candidate.to_path_buf());
        }
    }

    bail!(
        "not a Takt package: no {} found from {} upward",
        ROOT_MANIFEST_FILENAME,
        start.display()
    )
}

fn resolve_manifest_path(
    root: &Path,
    directory: &str,
    selector: &str,
    label: &str,
) -> Result<PathBuf> {
    let selector_path = Path::new(selector);
    if selector_path.exists() {
        return Ok(fs::canonicalize(selector_path)?);
    }

    let candidate =
        root.join(directory)
            .join(format!("{}.{}", slugify(selector), MANIFEST_EXTENSION));
    if candidate.exists() {
        return Ok(candidate);
    }

    for path in list_manifest_files(root.join(directory))? {
        let value: Value = read_json_file(&path)?;
        if value
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == selector)
        {
            return Ok(path);
        }
    }

    bail!(
        "unable to find {} '{}' under {}/",
        label,
        selector,
        directory
    )
}

fn list_manifest_files(dir: PathBuf) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension == MANIFEST_EXTENSION)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_json_file<T>(path: &Path) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn write_json_value<T>(value: &T, path: &Path) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn load_registry_config(repo: &Repository) -> Result<RegistryConfig> {
    let mut config = RegistryConfig {
        default_registry: normalize_registry_url("https://registry.npmjs.org/"),
        scoped_registries: BTreeMap::new(),
        auth_tokens: BTreeMap::new(),
    };

    if let Some(home) = env::var_os("HOME") {
        let user_npmrc = PathBuf::from(home).join(".npmrc");
        if user_npmrc.exists() {
            merge_npmrc_file(&mut config, &user_npmrc)?;
        }
    }

    let project_npmrc = repo.root.join(".npmrc");
    if project_npmrc.exists() {
        merge_npmrc_file(&mut config, &project_npmrc)?;
    }

    Ok(config)
}

fn merge_npmrc_file(config: &mut RegistryConfig, path: &Path) -> Result<()> {
    for raw_line in fs::read_to_string(path)?.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let value = interpolate_env_vars(raw_value.trim())?;

        if key == "registry" {
            config.default_registry = normalize_registry_url(&value);
        } else if key.starts_with('@') && key.ends_with(":registry") {
            let scope = key.trim_end_matches(":registry");
            config
                .scoped_registries
                .insert(scope.to_string(), normalize_registry_url(&value));
        } else if key.starts_with("//") && key.ends_with(":_authToken") {
            let host = key
                .trim_start_matches("//")
                .trim_end_matches(":_authToken")
                .to_string();
            config.auth_tokens.insert(host, value);
        }
    }

    Ok(())
}

fn interpolate_env_vars(value: &str) -> Result<String> {
    let mut rendered = value.to_string();
    while let Some(start) = rendered.find("${") {
        let rest = &rendered[start + 2..];
        let Some(end_offset) = rest.find('}') else {
            bail!("unterminated environment variable in npmrc value '{value}'");
        };
        let key = &rest[..end_offset];
        let env_value = env::var(key)
            .map_err(|_| eyre!("environment variable '{key}' referenced in .npmrc is not set"))?;
        rendered.replace_range(start..start + 3 + end_offset, &env_value);
    }
    Ok(rendered)
}

fn normalize_registry_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

fn registry_url_for_package<'a>(config: &'a RegistryConfig, package: &str) -> &'a str {
    package
        .split_once('/')
        .and_then(|(scope, _)| config.scoped_registries.get(scope))
        .map(String::as_str)
        .unwrap_or(&config.default_registry)
}

fn auth_token_for_url<'a>(config: &'a RegistryConfig, url: &str) -> Option<&'a str> {
    let key = registry_auth_key(url);
    config
        .auth_tokens
        .iter()
        .filter(|(candidate, _)| key.starts_with(candidate.as_str()))
        .max_by_key(|(candidate, _)| candidate.len())
        .map(|(_, token)| token.as_str())
}

fn registry_auth_key(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string()
}

fn fetch_registry_package_document(
    config: &RegistryConfig,
    package: &str,
) -> Result<NpmPackageDocument> {
    let registry = registry_url_for_package(config, package);
    let url = format!("{registry}{}", escape_registry_package_name(package));
    let client = Client::builder().build()?;
    let mut request = client.get(&url).header(
        "accept",
        "application/vnd.npm.install-v1+json, application/json",
    );
    if let Some(token) = auth_token_for_url(config, registry) {
        request = request.bearer_auth(token);
    }
    let response = request.send()?;
    if !response.status().is_success() {
        bail!(
            "failed to resolve dependency '{}': registry returned {}",
            package,
            response.status()
        );
    }
    Ok(response.json()?)
}

fn escape_registry_package_name(name: &str) -> String {
    name.replace('/', "%2f")
}

fn select_dependency_version(
    package: &str,
    specifier: &str,
    document: &NpmPackageDocument,
) -> Result<String> {
    if let Some(version) = document.dist_tags.get(specifier) {
        return Ok(version.clone());
    }

    if document.versions.contains_key(specifier) {
        return Ok(specifier.to_string());
    }

    let requirement = VersionReq::parse(specifier).map_err(|error| {
        eyre!(
            "dependency '{}' has unsupported specifier '{}': {error}",
            package,
            specifier
        )
    })?;

    let selected = document
        .versions
        .keys()
        .filter_map(|version| Version::parse(version).ok())
        .filter(|version| requirement.matches(version))
        .max()
        .ok_or_else(|| {
            eyre!(
                "dependency '{}' has no versions matching '{}'",
                package,
                specifier
            )
        })?;

    Ok(selected.to_string())
}

fn resolve_and_materialize_dependency(
    repo: &Repository,
    registry: &RegistryConfig,
    package: &str,
    specifier: &str,
    force: bool,
) -> Result<(String, String, String, PathBuf)> {
    let document = fetch_registry_package_document(registry, package)?;
    let version = select_dependency_version(package, specifier, &document)?;
    let version_document = document.versions.get(&version).ok_or_else(|| {
        eyre!(
            "registry metadata for '{}' is missing version '{}'",
            package,
            version
        )
    })?;
    let integrity = version_document.dist.integrity.clone().ok_or_else(|| {
        eyre!(
            "registry metadata for '{}@{}' is missing integrity",
            package,
            version
        )
    })?;
    let resolved = version_document.dist.tarball.clone();
    let package_path = ensure_cached_package_view(
        repo, registry, package, specifier, &version, &resolved, &integrity, force,
    )?;
    Ok((version, resolved, integrity, package_path))
}

fn ensure_cached_package_view(
    repo: &Repository,
    config: &RegistryConfig,
    package: &str,
    specifier: &str,
    version: &str,
    resolved: &str,
    integrity: &str,
    force: bool,
) -> Result<PathBuf> {
    let index = if !force {
        load_cached_package_index(&repo.store_root, package, version, Some(integrity))
    } else {
        None
    };
    let index = match index {
        Some(index) => index,
        None => fetch_registry_package_index(
            repo, config, package, specifier, version, resolved, integrity,
        )?,
    };

    materialize_package_view(repo, package, version, integrity, &index)
}

fn fetch_registry_package_index(
    repo: &Repository,
    config: &RegistryConfig,
    package: &str,
    specifier: &str,
    version: &str,
    resolved: &str,
    integrity: &str,
) -> Result<PackageIndex> {
    let client = Client::builder().build()?;
    let mut request = client.get(resolved);
    if let Some(token) = auth_token_for_url(config, resolved) {
        request = request.bearer_auth(token);
    }
    let mut response = request.send()?;
    if !response.status().is_success() {
        bail!(
            "failed to download '{}@{}' for '{}' from {}: {}",
            package,
            version,
            specifier,
            resolved,
            response.status()
        );
    }

    let mut tarball = Vec::new();
    response.read_to_end(&mut tarball)?;
    verify_package_integrity(&tarball, integrity)?;
    let index = import_npm_tarball_into_store(&repo.store_root, &tarball)?;

    if !index.contains_key(ROOT_MANIFEST_FILENAME) {
        bail!(
            "downloaded dependency '{}@{}' does not contain {}",
            package,
            version,
            ROOT_MANIFEST_FILENAME
        );
    }

    save_cached_package_index(&repo.store_root, package, version, Some(integrity), &index)?;
    Ok(index)
}

fn verify_package_integrity(bytes: &[u8], integrity: &str) -> Result<()> {
    let Some((algorithm, digest)) = integrity.split_once('-') else {
        bail!("unsupported integrity string '{integrity}'");
    };
    let expected = base64::engine::general_purpose::STANDARD.decode(digest)?;
    let actual = match algorithm {
        "sha256" => Sha256::digest(bytes).to_vec(),
        "sha384" => Sha384::digest(bytes).to_vec(),
        "sha512" => Sha512::digest(bytes).to_vec(),
        other => bail!("unsupported integrity algorithm '{other}'"),
    };

    if actual != expected {
        bail!("downloaded package failed integrity verification");
    }
    Ok(())
}

fn import_npm_tarball_into_store(store_root: &Path, bytes: &[u8]) -> Result<PackageIndex> {
    fs::create_dir_all(store_root)?;
    let decoder = GzDecoder::new(bytes);
    let mut archive = Archive::new(decoder);
    let mut index = PackageIndex::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir()
            || matches!(
                entry_type,
                tar::EntryType::XGlobalHeader
                    | tar::EntryType::XHeader
                    | tar::EntryType::GNULongName
                    | tar::EntryType::GNULongLink
            )
        {
            continue;
        }
        if !matches!(
            entry_type,
            tar::EntryType::Regular | tar::EntryType::Continuous
        ) {
            bail!("tarball entry type {entry_type:?} is not supported");
        }

        let raw_path = entry.path()?.to_path_buf();
        let Some(relative) = normalize_tar_entry_path(&raw_path)? else {
            continue;
        };

        let mode = entry.header().mode().unwrap_or(0o644);
        let executable = mode & 0o111 != 0;
        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;

        let stored = import_store_file(store_root, &content, executable)?;
        if index.insert(relative.clone(), stored).is_some() {
            bail!("tarball contains duplicate path '{}'", relative);
        }
    }

    Ok(index)
}

fn normalize_tar_entry_path(path: &Path) -> Result<Option<String>> {
    let mut components = path.components();
    let Some(first) = components.next() else {
        return Ok(None);
    };
    if first.as_os_str() != "package" {
        return Ok(None);
    }

    let relative = components.as_path();
    if relative.as_os_str().is_empty() {
        return Ok(None);
    }
    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        bail!("tarball contains an unsafe path '{}'", relative.display());
    }

    Ok(Some(relative.to_string_lossy().replace('\\', "/")))
}

fn import_store_file(store_root: &Path, content: &[u8], executable: bool) -> Result<StoredFile> {
    let hex_hash = blake3::hash(content).to_hex().to_string();
    let store_path = store_file_path(store_root, &hex_hash);
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if store_path.exists() && store_path.metadata()?.len() != content.len() as u64 {
        fs::remove_file(&store_path)?;
    }

    if !store_path.exists() {
        use std::io::Write;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&store_path)
        {
            Ok(mut file) => file.write_all(content)?,
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists
                    && store_path.metadata()?.len() == content.len() as u64 => {}
            Err(error) => return Err(error.into()),
        }
    }

    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = store_path.metadata()?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&store_path, permissions)?;
    }

    Ok(StoredFile {
        hex_hash,
        store_path,
        executable,
        size: content.len() as u64,
    })
}

fn store_file_path(store_root: &Path, hex_hash: &str) -> PathBuf {
    let (shard, suffix) = hex_hash.split_at(2);
    store_root.join(shard).join(suffix)
}

fn load_cached_package_index(
    store_root: &Path,
    name: &str,
    version: &str,
    integrity: Option<&str>,
) -> Option<PackageIndex> {
    let index_path = package_index_path(store_root, name, version, integrity).ok()?;
    let bytes = fs::read(&index_path).ok()?;
    let index: PackageIndex = serde_json::from_slice(&bytes).ok()?;
    if package_index_is_stale(&index) {
        let _ = fs::remove_file(index_path);
        return None;
    }
    Some(index)
}

fn save_cached_package_index(
    store_root: &Path,
    name: &str,
    version: &str,
    integrity: Option<&str>,
    index: &PackageIndex,
) -> Result<()> {
    let index_path = package_index_path(store_root, name, version, integrity)?;
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(index_path, serde_json::to_vec_pretty(index)?)?;
    Ok(())
}

fn package_index_is_stale(index: &PackageIndex) -> bool {
    !index.values().all(|stored| {
        stored
            .store_path
            .metadata()
            .map(|metadata| metadata.len() == stored.size)
            .unwrap_or(false)
    })
}

fn package_index_path(
    store_root: &Path,
    name: &str,
    version: &str,
    integrity: Option<&str>,
) -> Result<PathBuf> {
    let filename = format!(
        "{}@{}.json",
        encode_store_component(name),
        encode_store_component(version)
    );
    let index_root = store_index_root(store_root);
    if let Some(integrity) = integrity {
        let hex = integrity_to_hex(integrity)?;
        let shard = &hex[..16.min(hex.len())];
        Ok(index_root.join(shard).join(filename))
    } else {
        Ok(index_root.join(filename))
    }
}

fn materialize_package_view(
    repo: &Repository,
    package: &str,
    version: &str,
    integrity: &str,
    index: &PackageIndex,
) -> Result<PathBuf> {
    let package_path =
        materialized_package_view_path(&repo.cache_root, package, version, integrity);
    if package_path.join(ROOT_MANIFEST_FILENAME).exists() {
        return Ok(package_path);
    }
    if package_path.exists() {
        fs::remove_dir_all(&package_path)?;
    }

    let staging_path = package_path.with_extension(format!("tmp-{}", now_unix_ms()?));
    if staging_path.exists() {
        fs::remove_dir_all(&staging_path)?;
    }
    fs::create_dir_all(&staging_path)?;

    for (relative_path, stored) in index {
        let target_path = staging_path.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        link_or_copy_file(&stored.store_path, &target_path, stored.executable)?;
    }

    if !staging_path.join(ROOT_MANIFEST_FILENAME).exists() {
        bail!(
            "cached package '{}' at version '{}' does not contain {}",
            package,
            version,
            ROOT_MANIFEST_FILENAME
        );
    }

    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(&staging_path, &package_path) {
        Ok(()) => Ok(package_path),
        Err(_error) if package_path.exists() => {
            let _ = fs::remove_dir_all(&staging_path);
            Ok(package_path)
        }
        Err(error) => Err(error.into()),
    }
}

fn resolve_store_root(repo_root: &Path) -> PathBuf {
    let repo_local_store = repo_root
        .join(".takt")
        .join("store")
        .join(STORE_VERSION)
        .join(STORE_FILES_SUBDIR);
    if repo_local_store.exists() {
        return repo_local_store;
    }

    resolve_store_root_from(
        Some(repo_root),
        env::var_os(STORE_DIRECTORY_ENV).map(PathBuf::from),
        env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
    )
}

fn resolve_store_root_from(
    repo_root: Option<&Path>,
    configured_store_root: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
    _local_app_data: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = configured_store_root.filter(|path| !path.as_os_str().is_empty()) {
        return path;
    }

    #[cfg(windows)]
    if let Some(path) = _local_app_data.filter(|path| !path.as_os_str().is_empty()) {
        return path
            .join(CACHE_DIRECTORY_NAME)
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
    }

    if let Some(path) = xdg_data_home.filter(|path| !path.as_os_str().is_empty()) {
        return path
            .join(CACHE_DIRECTORY_NAME)
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
    }

    if let Some(path) = home.filter(|path| !path.as_os_str().is_empty()) {
        return path
            .join(".local")
            .join("share")
            .join(CACHE_DIRECTORY_NAME)
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
    }

    repo_root
        .map(|root| {
            root.join(".takt")
                .join("store")
                .join(STORE_VERSION)
                .join(STORE_FILES_SUBDIR)
        })
        .unwrap_or_else(|| {
            PathBuf::from(".takt")
                .join("store")
                .join(STORE_VERSION)
                .join(STORE_FILES_SUBDIR)
        })
}

fn resolve_cache_root(repo_root: &Path) -> PathBuf {
    let repo_local_cache = repo_root.join(".takt").join("cache");
    if repo_local_cache.exists() {
        return repo_local_cache;
    }

    resolve_cache_root_from(
        env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
    )
}

fn resolve_cache_root_from(
    xdg_cache_home: Option<PathBuf>,
    home: Option<PathBuf>,
    _local_app_data: Option<PathBuf>,
) -> PathBuf {
    #[cfg(windows)]
    if let Some(path) = _local_app_data.filter(|path| !path.as_os_str().is_empty()) {
        return path.join(CACHE_DIRECTORY_NAME);
    }

    if let Some(path) = xdg_cache_home.filter(|path| !path.as_os_str().is_empty()) {
        return path.join(CACHE_DIRECTORY_NAME);
    }

    if let Some(path) = home.filter(|path| !path.as_os_str().is_empty()) {
        return path.join(".cache").join(CACHE_DIRECTORY_NAME);
    }

    PathBuf::from(".takt").join("cache")
}

fn store_index_root(store_root: &Path) -> PathBuf {
    store_root
        .parent()
        .unwrap_or(store_root)
        .join(STORE_INDEX_SUBDIR)
}

fn virtual_store_root(cache_root: &Path) -> PathBuf {
    cache_root.join(STORE_VIEWS_SUBDIR)
}

fn materialized_package_view_path(
    cache_root: &Path,
    package: &str,
    version: &str,
    integrity: &str,
) -> PathBuf {
    let key = blake3::hash(format!("{package}\n{version}\n{integrity}\n").as_bytes())
        .to_hex()
        .to_string();
    virtual_store_root(cache_root).join(format!("{}-{}", encode_store_component(package), key))
}

fn integrity_to_hex(integrity: &str) -> Result<String> {
    let Some((_, digest)) = integrity.split_once('-') else {
        bail!("unsupported integrity string '{integrity}'");
    };
    Ok(base64::engine::general_purpose::STANDARD
        .decode(digest)?
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn encode_store_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let character = byte as char;
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            encoded.push(character);
        } else {
            encoded.push('_');
            encoded.push_str(&format!("{byte:02x}"));
        }
    }
    encoded
}

fn link_or_copy_file(source: &Path, destination: &Path, executable: bool) -> Result<()> {
    if fs::hard_link(source, destination).is_err() {
        fs::copy(source, destination)?;
    }

    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = destination.metadata()?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination, permissions)?;
    }

    Ok(())
}

fn load_installed_package_json(store_path: &Path) -> Result<PackageJsonManifest> {
    let package_json_path = store_path.join(PACKAGE_JSON_FILENAME);
    if package_json_path.exists() {
        read_json_file(&package_json_path)
    } else {
        let package_manifest: PackageManifest =
            read_json_file(&store_path.join(ROOT_MANIFEST_FILENAME))?;
        Ok(PackageJsonManifest::from_package_manifest(
            &package_manifest,
        ))
    }
}

fn dependency_specifier_matches_version(specifier: &str, version: &str) -> bool {
    if specifier == "latest" || specifier == version {
        return true;
    }

    match (VersionReq::parse(specifier), Version::parse(version)) {
        (Ok(requirement), Ok(version)) => requirement.matches(&version),
        _ => false,
    }
}

fn project_dependency_skills(
    repo: &Repository,
    dependencies: &[InstalledDependency],
) -> Result<BTreeMap<String, Vec<PathBuf>>> {
    let skills_root = repo.root.join(".agents").join("skills");
    fs::create_dir_all(&skills_root)?;
    clear_projected_skill_links(&skills_root)?;

    let mut projected = BTreeMap::new();
    for dependency in dependencies {
        let dependency_skills_root = dependency.store_path.join(".agents").join("skills");
        if !dependency_skills_root.exists() {
            continue;
        }

        let mut skill_paths = Vec::new();
        for entry in fs::read_dir(&dependency_skills_root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() || !path.join("SKILL.md").exists() {
                continue;
            }

            let destination = skills_root.join(format!(
                "takt-dep--{}--{}",
                sanitize_package_name(&dependency.name),
                entry.file_name().to_string_lossy()
            ));
            link_or_copy_directory(&path, &destination)?;
            skill_paths.push(destination);
        }

        projected.insert(dependency.name.clone(), skill_paths);
    }

    Ok(projected)
}

fn clear_projected_skill_links(skills_root: &Path) -> Result<()> {
    if !skills_root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(skills_root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("takt-dep--") {
            continue;
        }

        if path.is_dir() && !path.is_symlink() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn sanitize_package_name(value: &str) -> String {
    value
        .trim_start_matches('@')
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn link_or_copy_directory(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        if destination.is_dir() && !destination.is_symlink() {
            fs::remove_dir_all(destination)?;
        } else {
            fs::remove_file(destination)?;
        }
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(source, destination).is_ok() {
            return Ok(());
        }
    }

    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(source, destination).is_ok() {
            return Ok(());
        }
    }

    copy_directory_recursive(source, destination)
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn build_publish_tarball(repo: &Repository) -> Result<PathBuf> {
    let tarball_name = format!(
        "{}-{}.tgz",
        sanitize_package_name(&repo.package_json.name),
        repo.package_json.version
    );
    let tarball_path = repo.root.join(".takt").join("dist").join(tarball_name);
    if let Some(parent) = tarball_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if tarball_path.exists() {
        fs::remove_file(&tarball_path)?;
    }

    let file = fs::File::create(&tarball_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    for relative_path in publishable_paths(repo)? {
        append_publish_path(&mut builder, &repo.root, &relative_path)?;
    }
    let encoder = builder.into_inner()?;
    encoder.finish()?;

    Ok(tarball_path)
}

fn publishable_paths(repo: &Repository) -> Result<Vec<PathBuf>> {
    let mut include_roots = if repo.package_json.files.is_empty() {
        PackageJsonManifest::from_package_manifest(&repo.package).files
    } else {
        repo.package_json.files.clone()
    };
    include_roots.push(PACKAGE_JSON_FILENAME.into());
    include_roots.push(ROOT_MANIFEST_FILENAME.into());

    let mut seen = BTreeSet::new();
    let mut paths = Vec::new();
    for include_root in include_roots {
        collect_publish_paths(&repo.root, Path::new(&include_root), &mut seen, &mut paths)?;
    }
    paths.sort();
    Ok(paths)
}

fn collect_publish_paths(
    root: &Path,
    relative_path: &Path,
    seen: &mut BTreeSet<String>,
    paths: &mut Vec<PathBuf>,
) -> Result<()> {
    if is_generated_projected_skill(relative_path) {
        return Ok(());
    }

    let absolute_path = root.join(relative_path);
    if !absolute_path.exists() {
        return Ok(());
    }

    if absolute_path.is_file() {
        let key = relative_path.to_string_lossy().to_string();
        if seen.insert(key) {
            paths.push(relative_path.to_path_buf());
        }
        return Ok(());
    }

    let mut entries = fs::read_dir(&absolute_path)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let child_relative = relative_path.join(entry.file_name());
        collect_publish_paths(root, &child_relative, seen, paths)?;
    }

    Ok(())
}

fn is_generated_projected_skill(relative_path: &Path) -> bool {
    let mut components = relative_path.components();
    let Some(first) = components.next() else {
        return false;
    };
    let Some(second) = components.next() else {
        return false;
    };
    let Some(third) = components.next() else {
        return false;
    };

    first.as_os_str() == ".agents"
        && second.as_os_str() == "skills"
        && third
            .as_os_str()
            .to_string_lossy()
            .starts_with("takt-dep--")
}

fn append_publish_path(
    builder: &mut Builder<GzEncoder<fs::File>>,
    root: &Path,
    relative_path: &Path,
) -> Result<()> {
    let absolute_path = root.join(relative_path);
    let mut file = fs::File::open(&absolute_path)?;
    let metadata = file.metadata()?;
    let mut header = Header::new_gnu();
    header.set_path(Path::new("package").join(relative_path))?;
    header.set_size(metadata.len());
    header.set_mode(0o644);
    header.set_cksum();
    builder.append(&header, &mut file)?;
    Ok(())
}

fn npm_publish_arguments(
    tarball_path: &Path,
    tag: Option<&str>,
    access: Option<PublishAccess>,
    dry_run: bool,
) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("publish"),
        tarball_path.as_os_str().to_os_string(),
    ];
    if let Some(tag) = tag {
        arguments.push(OsString::from("--tag"));
        arguments.push(OsString::from(tag));
    }
    if let Some(access) = access {
        arguments.push(OsString::from("--access"));
        arguments.push(OsString::from(access.as_str()));
    }
    if dry_run {
        arguments.push(OsString::from("--dry-run"));
    }

    arguments
}

fn validation_report(
    kind: &str,
    subject: String,
    path: PathBuf,
    checks: Vec<ValidationCheck>,
) -> ValidationReport {
    let passed = checks.iter().all(|check| check.passed);
    ValidationReport {
        kind: kind.into(),
        subject,
        path,
        checks,
        passed,
    }
}

fn simple_check(
    name: impl Into<String>,
    passed: bool,
    message: impl Into<String>,
) -> ValidationCheck {
    let message = message.into();
    ValidationCheck {
        name: name.into(),
        passed,
        message: (!message.is_empty()).then_some(message),
    }
}

fn expect_equal(
    name: impl Into<String>,
    actual: &str,
    expected: &str,
    label: &str,
) -> ValidationCheck {
    simple_check(
        name,
        actual == expected,
        if actual == expected {
            String::new()
        } else {
            format!("{label} should be '{expected}', found '{actual}'")
        },
    )
}

fn workflow_has_cycle(workflow: &WorkflowDefinition) -> bool {
    fn visit(
        node: &str,
        workflow: &WorkflowDefinition,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visited.contains(node) {
            return false;
        }
        if !visiting.insert(node.to_string()) {
            return true;
        }

        if let Some(step) = workflow.steps.iter().find(|step| step.name == node) {
            for dependency in &step.needs {
                if visit(dependency, workflow, visiting, visited) {
                    return true;
                }
            }
        }

        visiting.remove(node);
        visited.insert(node.to_string());
        false
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    workflow
        .steps
        .iter()
        .any(|step| visit(&step.name, workflow, &mut visiting, &mut visited))
}

fn resolve_capability_reference(repo: &Repository, reference: &str) -> CapabilityResolution {
    if let Some((package, capability)) = reference.split_once('#') {
        if package.trim().is_empty() || capability.trim().is_empty() {
            return CapabilityResolution::Invalid {
                reference: reference.into(),
                reason: "package capability references must look like @scope/name#capability"
                    .into(),
            };
        }

        if package == repo.package.name {
            return repo
                .package
                .capabilities
                .get(capability)
                .map(|definition| CapabilityResolution::Local {
                    reference: reference.into(),
                    package: package.into(),
                    capability: capability.into(),
                    node: repo.package.node.clone(),
                    handler: definition.handler.clone(),
                })
                .unwrap_or_else(|| CapabilityResolution::MissingLocal {
                    reference: reference.into(),
                });
        }

        return resolve_external_capability(repo, reference, package, capability);
    }

    repo.package
        .capabilities
        .get(reference)
        .map(|definition| CapabilityResolution::Local {
            reference: reference.into(),
            package: repo.package.name.clone(),
            capability: reference.into(),
            node: repo.package.node.clone(),
            handler: definition.handler.clone(),
        })
        .unwrap_or_else(|| CapabilityResolution::MissingLocal {
            reference: reference.into(),
        })
}

fn resolve_external_capability(
    repo: &Repository,
    reference: &str,
    package: &str,
    capability: &str,
) -> CapabilityResolution {
    let Some(specifier) = repo.package_json.dependencies.get(package) else {
        return CapabilityResolution::MissingDependency {
            reference: reference.into(),
            package: package.into(),
        };
    };

    let Some(locked_package) = repo.lockfile.packages.get(package) else {
        return CapabilityResolution::UninstalledDependency {
            reference: reference.into(),
            package: package.into(),
            specifier: specifier.clone(),
        };
    };

    let Some(index) = load_cached_package_index(
        &repo.store_root,
        package,
        &locked_package.version,
        Some(&locked_package.integrity),
    ) else {
        return CapabilityResolution::UninstalledDependency {
            reference: reference.into(),
            package: package.into(),
            specifier: specifier.clone(),
        };
    };

    let store_path = match materialize_package_view(
        repo,
        package,
        &locked_package.version,
        &locked_package.integrity,
        &index,
    ) {
        Ok(store_path) => store_path,
        Err(error) => {
            return CapabilityResolution::Invalid {
                reference: reference.into(),
                reason: format!("failed to materialize dependency '{}': {error}", package),
            };
        }
    };
    let manifest_path = store_path.join(ROOT_MANIFEST_FILENAME);

    let Ok(package_manifest) = read_json_file::<PackageManifest>(&manifest_path) else {
        return CapabilityResolution::Invalid {
            reference: reference.into(),
            reason: format!(
                "installed dependency '{}' has an invalid {}",
                package, ROOT_MANIFEST_FILENAME
            ),
        };
    };

    package_manifest
        .capabilities
        .get(capability)
        .map(|definition| CapabilityResolution::InstalledExternal {
            reference: reference.into(),
            package: package.into(),
            specifier: specifier.clone(),
            version: locked_package.version.clone(),
            capability: capability.into(),
            node: package_manifest.node.clone(),
            handler: definition.handler.clone(),
            manifest_path: manifest_path.clone(),
            store_path: store_path.clone(),
        })
        .unwrap_or_else(|| CapabilityResolution::MissingExternalCapability {
            reference: reference.into(),
            package: package.into(),
            version: locked_package.version.clone(),
            capability: capability.into(),
            manifest_path,
        })
}

fn capability_resolution_error(resolution: &CapabilityResolution) -> Option<String> {
    match resolution {
        CapabilityResolution::Local { .. } | CapabilityResolution::InstalledExternal { .. } => None,
        CapabilityResolution::MissingLocal { reference } => {
            Some(format!("unresolved local capability '{reference}'"))
        }
        CapabilityResolution::MissingDependency { package, .. } => Some(format!(
            "dependency '{}' is not declared in {}",
            package, PACKAGE_JSON_FILENAME
        )),
        CapabilityResolution::UninstalledDependency {
            package, specifier, ..
        } => Some(format!(
            "dependency '{}' is declared as '{}' but is not installed; run `takt install`",
            package, specifier
        )),
        CapabilityResolution::MissingExternalCapability {
            package,
            version,
            capability,
            ..
        } => Some(format!(
            "installed dependency '{}' at version '{}' does not export capability '{}'",
            package, version, capability
        )),
        CapabilityResolution::Invalid { reason, .. } => Some(reason.clone()),
    }
}

fn persist_run_record<T>(repo_root: &Path, run_id: &str, record: &T) -> Result<PathBuf>
where
    T: Serialize,
{
    let path = repo_root
        .join(".takt")
        .join("runs")
        .join(format!("{run_id}.json"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(record)?)?;
    Ok(path)
}

fn now_unix_ms() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| eyre!("system clock is before UNIX_EPOCH: {error}"))?;
    Ok(duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityResolution, PublishAccess, ROOT_MANIFEST_FILENAME, STORE_FILES_SUBDIR,
        STORE_VERSION, TaktLockfile, build_publish_tarball, discover_repository, import_store_file,
        npm_publish_arguments, save_cached_package_index, validate_action_document,
        write_json_value,
    };
    use crate::core::plan_action_run;
    use crate::domain::{
        ActionDefinition, CapabilityDefinition, HandlerDefinition, LockedPackage,
        PackageJsonManifest, PackageManifest, SchemaReference,
    };
    use base64::Engine;
    use flate2::read::GzDecoder;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tar::Archive;
    use tempfile::tempdir;

    #[test]
    fn resolves_installed_external_capability_from_store() -> color_eyre::eyre::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();
        let store_root = root
            .join(".takt")
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
        let cache_root = root.join(".takt").join("cache");
        fs::create_dir_all(&store_root)?;
        fs::create_dir_all(&cache_root)?;

        let root_package = PackageManifest::starter("@acme/root".into(), Some("Root".into()));
        write_json_value(&root_package, &root.join(ROOT_MANIFEST_FILENAME))?;
        write_json_value(
            &PackageJsonManifest::starter(
                "@acme/root".into(),
                "0.1.0".into(),
                Some("Root".into()),
                BTreeMap::from([("@acme/dep".into(), "^1.0.0".into())]),
            ),
            &root.join("package.json"),
        )?;

        let dependency_package = PackageManifest {
            name: "@acme/dep".into(),
            version: "1.2.3".into(),
            description: Some("Dependency".into()),
            capabilities: BTreeMap::from([(
                "echo.run".into(),
                CapabilityDefinition {
                    description: Some("Echo".into()),
                    handler: HandlerDefinition {
                        entrypoint: "handlers/echo.mjs".into(),
                        argv: vec![],
                    },
                    input: SchemaReference {
                        path: "schemas/input.json".into(),
                        description: None,
                    },
                    output: SchemaReference {
                        path: "schemas/output.json".into(),
                        description: None,
                    },
                    permissions: Default::default(),
                },
            )]),
            ..PackageManifest::starter("@acme/dep".into(), Some("Dependency".into()))
        };
        let manifest_bytes = serde_json::to_vec_pretty(&dependency_package)?;
        let stored_manifest = import_store_file(&store_root, &manifest_bytes, false)?;
        let integrity = format!(
            "sha512-{}",
            base64::engine::general_purpose::STANDARD.encode([0_u8; 64])
        );
        save_cached_package_index(
            &store_root,
            "@acme/dep",
            "1.2.3",
            Some(&integrity),
            &BTreeMap::from([(ROOT_MANIFEST_FILENAME.into(), stored_manifest)]),
        )?;

        write_json_value(
            &TaktLockfile {
                lockfile_version: 1,
                packages: BTreeMap::from([(
                    "@acme/dep".into(),
                    LockedPackage {
                        specifier: "^1.0.0".into(),
                        version: "1.2.3".into(),
                        resolved: "https://registry.example.test/@acme/dep/-/dep-1.2.3.tgz".into(),
                        integrity,
                    },
                )]),
            },
            &root.join("takt.lock.json"),
        )?;

        fs::create_dir_all(root.join("actions"))?;
        write_json_value(
            &ActionDefinition::starter("echo".into(), "@acme/dep#echo.run".into()),
            &root.join("actions/echo.json"),
        )?;

        let repo = discover_repository(root)?;
        let action = super::load_action(&repo, "echo")?;
        let validation = validate_action_document(&repo, &action);
        assert!(validation.passed, "{validation:?}");

        let output = plan_action_run(&repo, "echo", BTreeMap::new(), false)?;
        match output.run.action.resolution {
            CapabilityResolution::InstalledExternal {
                package,
                version,
                capability,
                ..
            } => {
                assert_eq!(package, "@acme/dep");
                assert_eq!(version, "1.2.3");
                assert_eq!(capability, "echo.run");
            }
            other => panic!("expected installed external capability, found {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn publish_tarball_includes_takt_files_and_skills() -> color_eyre::eyre::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        let package = PackageManifest::starter("@acme/root".into(), Some("Root".into()));
        write_json_value(&package, &root.join(ROOT_MANIFEST_FILENAME))?;
        write_json_value(
            &PackageJsonManifest::from_package_manifest(&package),
            &root.join("package.json"),
        )?;

        fs::create_dir_all(root.join("handlers"))?;
        fs::write(root.join("handlers/example.mjs"), "export default {};\n")?;
        fs::create_dir_all(root.join("schemas"))?;
        fs::write(root.join("schemas/example-input.json"), "{}\n")?;
        fs::write(root.join("schemas/example-output.json"), "{}\n")?;
        fs::create_dir_all(root.join(".agents/skills/demo"))?;
        fs::write(
            root.join(".agents/skills/demo/SKILL.md"),
            "---\nname: demo\n---\n",
        )?;
        fs::create_dir_all(root.join(".agents/skills/takt-dep--acme-dep--demo"))?;
        fs::write(
            root.join(".agents/skills/takt-dep--acme-dep--demo/SKILL.md"),
            "---\nname: projected\n---\n",
        )?;
        fs::write(root.join("README.md"), "# demo\n")?;

        let repo = discover_repository(root)?;
        let tarball_path = build_publish_tarball(&repo)?;

        let file = fs::File::open(tarball_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        let mut paths = archive
            .entries()?
            .map(|entry| entry.map(|entry| entry.path().unwrap().to_path_buf()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();

        assert!(paths.contains(&Path::new("package/takt.json").to_path_buf()));
        assert!(paths.contains(&Path::new("package/package.json").to_path_buf()));
        assert!(paths.contains(&Path::new("package/handlers/example.mjs").to_path_buf()));
        assert!(paths.contains(&Path::new("package/.agents/skills/demo/SKILL.md").to_path_buf()));
        assert!(!paths.contains(
            &Path::new("package/.agents/skills/takt-dep--acme-dep--demo/SKILL.md").to_path_buf()
        ));

        Ok(())
    }

    #[test]
    fn npm_publish_arguments_include_access_tag_and_dry_run() {
        let arguments = npm_publish_arguments(
            Path::new("/tmp/@acme-demo-0.1.0.tgz"),
            Some("next"),
            Some(PublishAccess::Public),
            true,
        )
        .into_iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

        assert_eq!(
            arguments,
            vec![
                "publish",
                "/tmp/@acme-demo-0.1.0.tgz",
                "--tag",
                "next",
                "--access",
                "public",
                "--dry-run",
            ]
        );
    }

    #[test]
    fn resolve_store_root_prefers_existing_repo_local_store() -> color_eyre::eyre::Result<()> {
        let temp = tempdir()?;
        let repo_local_store = temp
            .path()
            .join(".takt")
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
        fs::create_dir_all(&repo_local_store)?;

        assert_eq!(super::resolve_store_root(temp.path()), repo_local_store);
        Ok(())
    }

    #[test]
    fn resolve_store_root_from_prefers_configured_store_root() {
        let configured = PathBuf::from("/tmp/takt-global-store");
        let resolved = super::resolve_store_root_from(
            None,
            Some(configured.clone()),
            Some(PathBuf::from("/tmp/xdg-data")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/local-app-data")),
        );

        assert_eq!(resolved, configured);
    }

    #[test]
    fn resolve_store_root_from_uses_xdg_data_directory_when_set() {
        let resolved = super::resolve_store_root_from(
            None,
            None,
            Some(PathBuf::from("/tmp/xdg-data")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/local-app-data")),
        );

        assert_eq!(
            resolved,
            PathBuf::from("/tmp/xdg-data")
                .join("takt")
                .join("store")
                .join(STORE_VERSION)
                .join(STORE_FILES_SUBDIR)
        );
    }

    #[test]
    fn resolve_store_root_from_falls_back_to_home_data_directory() {
        let resolved = super::resolve_store_root_from(
            None,
            None,
            None,
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/local-app-data")),
        );

        assert_eq!(
            resolved,
            PathBuf::from("/tmp/home")
                .join(".local")
                .join("share")
                .join("takt")
                .join("store")
                .join(STORE_VERSION)
                .join(STORE_FILES_SUBDIR)
        );
    }

    #[test]
    fn resolve_cache_root_from_uses_xdg_cache_directory_when_set() {
        let resolved = super::resolve_cache_root_from(
            Some(PathBuf::from("/tmp/xdg-cache")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/local-app-data")),
        );

        assert_eq!(resolved, PathBuf::from("/tmp/xdg-cache").join("takt"));
    }

    #[test]
    fn resolve_cache_root_from_falls_back_to_home_cache_directory() {
        let resolved = super::resolve_cache_root_from(
            None,
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/local-app-data")),
        );

        assert_eq!(
            resolved,
            PathBuf::from("/tmp/home").join(".cache").join("takt")
        );
    }
}
