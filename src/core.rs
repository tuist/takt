use crate::datastore::{
    ArtifactRecord, ListArtifactsQuery, ListRunsQuery, RunKind, RunMode, RunRecord, RunSource,
    RunStatus, open_repo_datastore,
};
use crate::execution::{ExecutionInput, execute_node_handler};
use crate::domain::{
    API_VERSION, ActionDefinition, CapabilityDefinition, DEFAULT_RUNTIME_NAME, HandlerDefinition,
    NetworkPolicy, PackageManifest, RuntimeProfile, SANDBOX_PROCESS, WorkflowDefinition,
};
use crate::scaffold::{CodingAgent, ScaffoldFile, package_bootstrap_files, package_project_root};
use clap::ValueEnum;
use color_eyre::eyre::{Result, bail, eyre};
use schemars::schema_for;
use schemars::{JsonSchema, Schema};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use crate::query::{new_run_id, now_unix_ms};
use std::fs;
use std::path::{Path, PathBuf};

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
    Run,
    Artifact,
    Config,
}

#[derive(Debug, Serialize)]
pub struct SchemaBundle {
    pub package: Schema,
    pub capability: Schema,
    pub action: Schema,
    pub workflow: Schema,
    pub run: Schema,
    pub artifact: Schema,
    pub config: Schema,
}

pub fn schema_bundle() -> SchemaBundle {
    SchemaBundle {
        package: schema_for!(PackageManifest),
        capability: schema_for!(CapabilityDefinition),
        action: schema_for!(ActionDefinition),
        workflow: schema_for!(WorkflowDefinition),
        run: schema_for!(RunRecord),
        artifact: schema_for!(ArtifactRecord),
        config: schema_for!(crate::config::RepoConfig),
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
        SchemaTarget::Run => {
            serde_json::to_value(schema_for!(RunRecord)).expect("run schema is valid")
        }
        SchemaTarget::Artifact => {
            serde_json::to_value(schema_for!(ArtifactRecord)).expect("artifact schema is valid")
        }
        SchemaTarget::Config => serde_json::to_value(schema_for!(crate::config::RepoConfig))
            .expect("config schema is valid"),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_unix_ms: Option<u64>,
    pub repo_root: PathBuf,
    pub persisted: bool,
    pub message: String,
    pub inputs: BTreeMap<String, Value>,
    pub validation: ValidationReport,
    pub action: ActionRunTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_ids: Vec<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_unix_ms: Option<u64>,
    pub repo_root: PathBuf,
    pub persisted: bool,
    pub message: String,
    pub inputs: BTreeMap<String, Value>,
    pub validation: ValidationReport,
    pub workflow: WorkflowRunTarget,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_run_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
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

pub fn list_runs(repo: &Repository, query: &ListRunsQuery) -> Result<Vec<RunRecord>> {
    let (_loaded, provider) = open_repo_datastore(&repo.root)?;
    provider.list_runs(query)
}

pub fn get_run(repo: &Repository, id: &str) -> Result<Option<RunRecord>> {
    let (_loaded, provider) = open_repo_datastore(&repo.root)?;
    provider.get_run(id)
}

pub fn list_artifacts(repo: &Repository, query: &ListArtifactsQuery) -> Result<Vec<ArtifactRecord>> {
    let (_loaded, provider) = open_repo_datastore(&repo.root)?;
    provider.list_artifacts(query)
}

pub fn get_artifact(repo: &Repository, id: &str) -> Result<Option<ArtifactRecord>> {
    let (_loaded, provider) = open_repo_datastore(&repo.root)?;
    provider.get_artifact(id)
}

#[derive(Debug, Default)]
pub struct RunListInput {
    pub kind: Option<RunKind>,
    pub status: Option<RunStatus>,
    pub since: Option<String>,
    pub limit: Option<usize>,
    /// Raw "path=value" strings; parsed and applied as AND-joined equality
    /// predicates against `RunRecord::lookup_path`.
    pub predicates: Vec<String>,
}

pub fn run_list_envelope(
    repo: &Repository,
    input: &RunListInput,
) -> Result<crate::query::ListEnvelope<RunRecord>> {
    let predicates: Vec<crate::query::Predicate> = input
        .predicates
        .iter()
        .map(|raw| crate::query::parse_predicate(raw))
        .collect::<Result<_>>()?;

    let query = ListRunsQuery {
        kind: input.kind,
        status: input.status,
        since_unix_ms: input
            .since
            .as_deref()
            .map(crate::query::since_threshold_unix_ms)
            .transpose()?,
    };
    let mut results = list_runs(repo, &query)?;
    if !predicates.is_empty() {
        results.retain(|run| {
            predicates.iter().all(|predicate| {
                run.lookup_path(&predicate.path)
                    .is_some_and(|actual| actual == predicate.value)
            })
        });
    }
    let total = results.len();
    if let Some(limit) = input.limit {
        results.truncate(limit);
    }
    Ok(crate::query::ListEnvelope::new("run list", total, results))
}

#[derive(Debug, Default)]
pub struct ArtifactListInput {
    pub run: Option<String>,
    pub name: Option<String>,
    pub capability: Option<String>,
    pub tags: BTreeMap<String, String>,
    pub since: Option<String>,
    pub limit: Option<usize>,
    /// Raw "path=value" strings; parsed and applied as AND-joined equality.
    pub predicates: Vec<String>,
}

pub fn artifact_list_envelope(
    repo: &Repository,
    input: &ArtifactListInput,
) -> Result<crate::query::ListEnvelope<ArtifactRecord>> {
    let predicates: Vec<crate::query::Predicate> = input
        .predicates
        .iter()
        .map(|raw| crate::query::parse_predicate(raw))
        .collect::<Result<_>>()?;

    let query = ListArtifactsQuery {
        run_id: input.run.clone(),
        name: input.name.clone(),
        capability: input.capability.clone(),
        tags: input.tags.clone(),
        since_unix_ms: input
            .since
            .as_deref()
            .map(crate::query::since_threshold_unix_ms)
            .transpose()?,
    };

    let mut results = list_artifacts(repo, &query)?;
    if !predicates.is_empty() {
        results.retain(|artifact| {
            predicates.iter().all(|predicate| {
                artifact
                    .lookup_path(&predicate.path)
                    .is_some_and(|actual| actual == predicate.value)
            })
        });
    }
    let total = results.len();
    if let Some(limit) = input.limit {
        results.truncate(limit);
    }
    Ok(crate::query::ListEnvelope::new(
        "artifact list",
        total,
        results,
    ))
}

pub fn plan_action_run(
    repo: &Repository,
    selector: &str,
    inputs: BTreeMap<String, Value>,
    persist: bool,
    source: RunSource,
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

    let (id, planned_at_unix_ms) = new_run_id()?;

    if persist {
        let (_loaded, provider) = open_repo_datastore(&repo.root)?;
        let run_record = RunRecord {
            id: id.clone(),
            kind: RunKind::Action,
            status: RunStatus::Planned,
            mode: RunMode::PlanOnly,
            source: source.clone(),
            started_at_unix_ms: planned_at_unix_ms,
            finished_at_unix_ms: None,
            repo_root: repo.root.clone(),
            inputs: inputs.clone(),
            target_name: document.action.name.clone(),
            target_path: document.path.clone(),
            artifact_ids: Vec::new(),
            child_run_ids: Vec::new(),
            output: None,
            error_message: None,
        };
        provider.put_run(&run_record)?;
    }

    Ok(ActionRunOutput {
        command: "run action",
        run: ActionRunRecord {
            id,
            status: RunStatus::Planned,
            mode: RunMode::PlanOnly,
            planned_at_unix_ms,
            finished_at_unix_ms: None,
            repo_root: repo.root.clone(),
            persisted: persist,
            message: "Plan-only mode: Takt validated and resolved the action without invoking the handler.".into(),
            inputs,
            validation,
            action: ActionRunTarget {
                name: document.action.name,
                path: document.path,
                capability: document.action.capability,
                resolution,
            },
            output: None,
            artifact_ids: Vec::new(),
        },
    })
}

pub fn execute_action_run(
    repo: &Repository,
    selector: &str,
    inputs: BTreeMap<String, Value>,
    persist: bool,
    source: RunSource,
) -> Result<ActionRunOutput> {
    let document = load_action(repo, selector)?;
    let validation = validate_action_document(repo, &document);
    if !validation.passed {
        bail!("action '{}' failed validation", document.action.name);
    }

    let resolution = resolve_capability_reference(repo, &document.action.capability);
    let (handler, capability_name) = match &resolution {
        CapabilityResolution::Local {
            handler,
            capability,
            ..
        } => (handler.clone(), capability.clone()),
        CapabilityResolution::MissingLocal { reference } => {
            bail!("cannot execute unresolved local capability '{reference}'")
        }
        CapabilityResolution::Invalid { reason, .. } => {
            bail!("cannot execute invalid capability reference: {reason}")
        }
        CapabilityResolution::External { package, capability } => bail!(
            "cannot execute external capability '{package}#{capability}': external resolution is not implemented yet"
        ),
    };

    let runtime = resolve_runtime_profile(&repo.package, &capability_name)?;

    let mut merged_inputs = document.action.with.clone();
    for (key, value) in &inputs {
        merged_inputs.insert(key.clone(), value.clone());
    }

    let (id, planned_at_unix_ms) = new_run_id()?;

    let (loaded_config, provider) = if persist {
        let pair = open_repo_datastore(&repo.root)?;
        (Some(pair.0), Some(pair.1))
    } else {
        (None, None)
    };
    let _ = loaded_config;

    let base_record = RunRecord {
        id: id.clone(),
        kind: RunKind::Action,
        status: RunStatus::Running,
        mode: RunMode::Execute,
        source: source.clone(),
        started_at_unix_ms: planned_at_unix_ms,
        finished_at_unix_ms: None,
        repo_root: repo.root.clone(),
        inputs: merged_inputs.clone(),
        target_name: document.action.name.clone(),
        target_path: document.path.clone(),
        artifact_ids: Vec::new(),
        child_run_ids: Vec::new(),
        output: None,
        error_message: None,
    };

    if let Some(provider) = provider.as_ref() {
        provider.put_run(&base_record)?;
    }

    let scratch_root = repo.root.join(".takt").join("datastore").join("runs-scratch");
    let blobs_root = repo.root.join(".takt").join("datastore").join("blobs");
    let execution_input = ExecutionInput {
        run_id: id.clone(),
        capability: capability_name.clone(),
        handler_entrypoint: PathBuf::from(handler.entrypoint),
        package_root: repo.root.clone(),
        inputs: merged_inputs.clone(),
        blobs_root,
        scratch_root,
        runtime,
    };

    let execution_result = execute_node_handler(execution_input);

    let finished_at_unix_ms = now_unix_ms()?;
    let (status, output, artifact_records, error_message, message) = match execution_result {
        Ok(outcome) => {
            let message = format!(
                "Handler '{}' completed successfully and emitted {} artifact(s).",
                capability_name,
                outcome.artifacts.len()
            );
            (
                RunStatus::Succeeded,
                outcome.output,
                outcome.artifacts,
                None,
                message,
            )
        }
        Err(error) => {
            let text = format!("{error:#}");
            (
                RunStatus::Failed,
                None,
                Vec::new(),
                Some(text.clone()),
                format!("Handler '{capability_name}' failed: {text}"),
            )
        }
    };

    let artifact_ids: Vec<String> = artifact_records.iter().map(|a| a.id.clone()).collect();

    if let Some(provider) = provider.as_ref() {
        for artifact in &artifact_records {
            provider.put_artifact(artifact)?;
        }
        let final_record = RunRecord {
            status,
            finished_at_unix_ms: Some(finished_at_unix_ms),
            artifact_ids: artifact_ids.clone(),
            output: output.clone(),
            error_message: error_message.clone(),
            ..base_record
        };
        provider.put_run(&final_record)?;
    }

    Ok(ActionRunOutput {
        command: "run action",
        run: ActionRunRecord {
            id,
            status,
            mode: RunMode::Execute,
            planned_at_unix_ms,
            finished_at_unix_ms: Some(finished_at_unix_ms),
            repo_root: repo.root.clone(),
            persisted: persist,
            message,
            inputs: merged_inputs,
            validation,
            action: ActionRunTarget {
                name: document.action.name,
                path: document.path,
                capability: document.action.capability,
                resolution,
            },
            output,
            artifact_ids,
        },
    })
}

pub fn plan_workflow_run(
    repo: &Repository,
    selector: &str,
    inputs: BTreeMap<String, Value>,
    persist: bool,
    source: RunSource,
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

    let (id, planned_at_unix_ms) = new_run_id()?;

    if persist {
        let (_loaded, provider) = open_repo_datastore(&repo.root)?;
        let run_record = RunRecord {
            id: id.clone(),
            kind: RunKind::Workflow,
            status: RunStatus::Planned,
            mode: RunMode::PlanOnly,
            source: source.clone(),
            started_at_unix_ms: planned_at_unix_ms,
            finished_at_unix_ms: None,
            repo_root: repo.root.clone(),
            inputs: inputs.clone(),
            target_name: document.workflow.name.clone(),
            target_path: document.path.clone(),
            artifact_ids: Vec::new(),
            child_run_ids: Vec::new(),
            output: None,
            error_message: None,
        };
        provider.put_run(&run_record)?;
    }

    Ok(WorkflowRunOutput {
        command: "run workflow",
        run: WorkflowRunRecord {
            id,
            status: RunStatus::Planned,
            mode: RunMode::PlanOnly,
            planned_at_unix_ms,
            finished_at_unix_ms: None,
            repo_root: repo.root.clone(),
            persisted: persist,
            message: "Plan-only mode: Takt validated and resolved the workflow without invoking any handler.".into(),
            inputs,
            validation,
            workflow: WorkflowRunTarget {
                name: document.workflow.name,
                path: document.path,
                steps,
            },
            child_run_ids: Vec::new(),
            output: None,
        },
    })
}

pub fn execute_workflow_run(
    repo: &Repository,
    selector: &str,
    inputs: BTreeMap<String, Value>,
    persist: bool,
    source: RunSource,
) -> Result<WorkflowRunOutput> {
    let document = load_workflow(repo, selector)?;
    let validation = validate_workflow_document(repo, &document);
    if !validation.passed {
        bail!("workflow '{}' failed validation", document.workflow.name);
    }

    for step in &document.workflow.steps {
        if step.foreach.is_some() {
            bail!(
                "workflow step '{}' uses `foreach` which is not implemented yet",
                step.name
            );
        }
        if step.if_expression.is_some() {
            bail!(
                "workflow step '{}' uses `if` which is not implemented yet",
                step.name
            );
        }
    }

    let ordered_steps = topological_step_order(&document.workflow)?;

    // Build the resolved step targets up front so the response carries the same
    // metadata that plan_workflow_run would return.
    let mut step_targets = Vec::with_capacity(document.workflow.steps.len());
    for step in &document.workflow.steps {
        let action_document = load_action(repo, &step.uses)?;
        let resolution = resolve_capability_reference(repo, &action_document.action.capability);
        step_targets.push(WorkflowStepRunTarget {
            name: step.name.clone(),
            action: action_document.action.name.clone(),
            action_path: action_document.path.clone(),
            capability: action_document.action.capability.clone(),
            resolution,
            needs: step.needs.clone(),
        });
    }

    let (workflow_run_id, planned_at_unix_ms) = new_run_id()?;

    let (_loaded, provider) = if persist {
        let pair = open_repo_datastore(&repo.root)?;
        (Some(pair.0), Some(pair.1))
    } else {
        (None, None)
    };

    let base_record = RunRecord {
        id: workflow_run_id.clone(),
        kind: RunKind::Workflow,
        status: RunStatus::Running,
        mode: RunMode::Execute,
        source: source.clone(),
        started_at_unix_ms: planned_at_unix_ms,
        finished_at_unix_ms: None,
        repo_root: repo.root.clone(),
        inputs: inputs.clone(),
        target_name: document.workflow.name.clone(),
        target_path: document.path.clone(),
        artifact_ids: Vec::new(),
        child_run_ids: Vec::new(),
        output: None,
        error_message: None,
    };
    if let Some(provider) = provider.as_ref() {
        provider.put_run(&base_record)?;
    }

    let mut child_run_ids: Vec<String> = Vec::new();
    let mut overall_status = RunStatus::Succeeded;
    let mut error_message: Option<String> = None;
    let mut message = format!(
        "Workflow '{}' completed all {} step(s).",
        document.workflow.name,
        ordered_steps.len()
    );

    let mut template_ctx = crate::template::TemplateContext {
        workflow_inputs: inputs.clone(),
        ..crate::template::TemplateContext::default()
    };

    for step in &ordered_steps {
        let expanded_with =
            match crate::template::expand_inputs(&step.with, &template_ctx) {
                Ok(map) => map,
                Err(error) => {
                    overall_status = RunStatus::Failed;
                    let text = format!("{error:#}");
                    error_message = Some(format!(
                        "step '{}' failed during template expansion: {}",
                        step.name, text
                    ));
                    message = format!(
                        "Workflow '{}' stopped because step '{}' had an invalid template expression.",
                        document.workflow.name, step.name
                    );
                    break;
                }
            };

        let mut step_inputs = inputs.clone();
        for (key, value) in expanded_with {
            step_inputs.insert(key, value);
        }

        let step_source = RunSource::Workflow {
            workflow_run_id: workflow_run_id.clone(),
            step_name: step.name.clone(),
        };

        let action_result =
            execute_action_run(repo, &step.uses, step_inputs, persist, step_source);
        match action_result {
            Ok(action_output) => {
                child_run_ids.push(action_output.run.id.clone());
                template_ctx.record_step(
                    &step.name,
                    action_output.run.id.clone(),
                    action_output.run.output.clone(),
                );
                if action_output.run.status == RunStatus::Failed {
                    overall_status = RunStatus::Failed;
                    error_message = Some(format!(
                        "step '{}' failed: {}",
                        step.name, action_output.run.message
                    ));
                    message = format!(
                        "Workflow '{}' stopped after step '{}' failed.",
                        document.workflow.name, step.name
                    );
                    break;
                }
            }
            Err(error) => {
                overall_status = RunStatus::Failed;
                let text = format!("{error:#}");
                error_message = Some(format!("step '{}' errored: {}", step.name, text));
                message = format!(
                    "Workflow '{}' stopped because step '{}' errored.",
                    document.workflow.name, step.name
                );
                break;
            }
        }
    }
    let step_outputs: BTreeMap<String, Value> = template_ctx
        .step_outputs
        .into_iter()
        .collect();

    let finished_at_unix_ms = now_unix_ms()?;
    let output_value = if step_outputs.is_empty() {
        None
    } else {
        Some(Value::Object(step_outputs.into_iter().collect()))
    };

    if let Some(provider) = provider.as_ref() {
        let final_record = RunRecord {
            status: overall_status,
            finished_at_unix_ms: Some(finished_at_unix_ms),
            child_run_ids: child_run_ids.clone(),
            output: output_value.clone(),
            error_message: error_message.clone(),
            ..base_record
        };
        provider.put_run(&final_record)?;
    }

    Ok(WorkflowRunOutput {
        command: "run workflow",
        run: WorkflowRunRecord {
            id: workflow_run_id,
            status: overall_status,
            mode: RunMode::Execute,
            planned_at_unix_ms,
            finished_at_unix_ms: Some(finished_at_unix_ms),
            repo_root: repo.root.clone(),
            persisted: persist,
            message,
            inputs,
            validation,
            workflow: WorkflowRunTarget {
                name: document.workflow.name,
                path: document.path,
                steps: step_targets,
            },
            child_run_ids,
            output: output_value,
        },
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

fn topological_step_order(workflow: &WorkflowDefinition) -> Result<Vec<crate::domain::WorkflowStep>> {
    use std::collections::BTreeSet;

    if workflow_has_cycle(workflow) {
        bail!("workflow steps contain a cycle and cannot be executed");
    }

    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut ordered: Vec<crate::domain::WorkflowStep> = Vec::with_capacity(workflow.steps.len());

    fn visit(
        name: &str,
        workflow: &WorkflowDefinition,
        visited: &mut BTreeSet<String>,
        ordered: &mut Vec<crate::domain::WorkflowStep>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Ok(());
        }
        let step = workflow
            .steps
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| eyre!("workflow references missing step '{}'", name))?;
        for dependency in &step.needs {
            visit(dependency, workflow, visited, ordered)?;
        }
        if visited.insert(name.to_string()) {
            ordered.push(step.clone());
        }
        Ok(())
    }

    for step in &workflow.steps {
        visit(&step.name, workflow, &mut visited, &mut ordered)?;
    }
    Ok(ordered)
}

fn resolve_runtime_profile(
    package: &PackageManifest,
    capability_name: &str,
) -> Result<RuntimeProfile> {
    let capability = package.capabilities.get(capability_name);
    let runtime_name = capability
        .and_then(|cap| cap.runtime.as_deref())
        .unwrap_or(DEFAULT_RUNTIME_NAME);

    if let Some(profile) = package.runtimes.get(runtime_name) {
        return Ok(profile.clone());
    }

    if runtime_name == DEFAULT_RUNTIME_NAME {
        return Ok(RuntimeProfile {
            sandbox: SANDBOX_PROCESS.into(),
            image: None,
            cpus: None,
            memory_mb: None,
            network: NetworkPolicy::default(),
        });
    }

    bail!(
        "capability '{}' references runtime profile '{}' but it is not declared in the package's `runtimes` table",
        capability_name,
        runtime_name
    )
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

