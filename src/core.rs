use crate::domain::{
    API_VERSION, ActionDefinition, CapabilityDefinition, HandlerDefinition, PackageManifest,
    WorkflowDefinition,
};
use crate::scaffold::{CodingAgent, ScaffoldFile, package_bootstrap_files, package_project_root};
use clap::ValueEnum;
use color_eyre::eyre::{Result, bail, eyre};
use schemars::schema_for;
use schemars::{JsonSchema, Schema};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CONCEPT_CHAIN: &str = "package -> capability -> action -> workflow -> run -> artifact";
pub const EXECUTION_RULE: &str =
    "packages pin an exact Node version; workflows never point at scripts directly.";
pub const ROOT_MANIFEST_FILENAME: &str = "takt.json";
pub const MANIFEST_EXTENSION: &str = "json";

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
    let mut files = vec![json_scaffold_file(&manifest, output, "package")?];
    files.extend(package_bootstrap_files(&project_root, &name, coding_agent));
    let written = write_scaffold_files(&files, force)?;

    Ok(InitOutput {
        command: "init",
        coding_agent,
        package: manifest,
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

#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
    pub package_path: PathBuf,
    pub package: PackageManifest,
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
    Ok(Repository {
        root,
        package_path,
        package,
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
    let mut checks = Vec::new();

    checks.push(expect_equal(
        "API version",
        &package.api_version,
        API_VERSION,
        "package manifest api_version",
    ));
    checks.push(simple_check(
        "Package name",
        !package.name.trim().is_empty(),
        "package name must not be empty",
    ));
    checks.push(simple_check(
        "Package version",
        !package.version.trim().is_empty(),
        "package version must not be empty",
    ));
    checks.push(simple_check(
        "Node version",
        !package.node.trim().is_empty(),
        "package node must not be empty",
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
        CapabilityResolution::External {
            package,
            capability,
        } => simple_check(
            format!(
                "External capability reference '{}#{}' is well-formed",
                package, capability
            ),
            true,
            "external package resolution is not implemented yet",
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
    let mut checks = Vec::new();

    checks.push(expect_equal(
        "API version",
        &workflow.api_version,
        API_VERSION,
        "workflow api_version",
    ));
    checks.push(expect_equal(
        "Kind",
        &workflow.kind,
        "Workflow",
        "workflow kind",
    ));
    checks.push(simple_check(
        "Workflow name",
        !workflow.name.trim().is_empty(),
        "workflow name must not be empty",
    ));
    checks.push(simple_check(
        "Workflow steps",
        !workflow.steps.is_empty(),
        "workflow must declare at least one step",
    ));

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
    External {
        package: String,
        capability: String,
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
    if let CapabilityResolution::MissingLocal { reference } = &resolution {
        bail!("cannot plan run for unresolved local capability '{reference}'");
    }
    if let CapabilityResolution::Invalid { reason, .. } = &resolution {
        bail!("cannot plan run for invalid capability reference: {reason}");
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
        if let CapabilityResolution::MissingLocal { reference } = &resolution {
            bail!(
                "workflow step '{}' references unresolved local capability '{}'",
                step.name,
                reference
            );
        }
        if let CapabilityResolution::Invalid { reason, .. } = &resolution {
            bail!(
                "workflow step '{}' references an invalid capability: {reason}",
                step.name
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

        return CapabilityResolution::External {
            package: package.into(),
            capability: capability.into(),
        };
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
