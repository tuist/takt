use crate::config::{
    DatastoreConfig, FILESYSTEM_PROVIDER, LoadedRepoConfig, load_repo_config,
    resolve_filesystem_datastore_root,
};
use crate::domain::{ArtifactType, RetentionPolicy, SchemaReference};
use color_eyre::eyre::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunKind {
    Action,
    Workflow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Planned,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunMode {
    PlanOnly,
    Execute,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProducerKind {
    Capability,
    Action,
    Workflow,
}

/// How a run came into existence. Lets agents distinguish ad-hoc CLI runs
/// from runs that were spawned as a workflow step.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RunSource {
    Cli,
    Mcp,
    Workflow {
        workflow_run_id: String,
        step_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum StorageRef {
    Inline { value: Value },
    File { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunRecord {
    pub id: String,
    pub kind: RunKind,
    pub status: RunStatus,
    pub mode: RunMode,
    pub source: RunSource,
    pub started_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_unix_ms: Option<u64>,
    pub repo_root: PathBuf,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, Value>,
    pub target_name: String,
    pub target_path: PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_ids: Vec<String>,
    /// Workflow runs use this to enumerate the per-step action run ids. Empty
    /// for action runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_run_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactRecord {
    pub id: String,
    pub run_id: String,
    pub producer_kind: ProducerKind,
    pub producer_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_name: Option<String>,
    pub name: String,
    pub artifact_type: ArtifactType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<SchemaReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub version: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
    pub created_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<RetentionPolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vary: Vec<String>,
    pub storage_ref: StorageRef,
}

#[derive(Debug, Clone, Default)]
pub struct ListRunsQuery {
    pub kind: Option<RunKind>,
    pub status: Option<RunStatus>,
    pub since_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct ListArtifactsQuery {
    pub run_id: Option<String>,
    pub name: Option<String>,
    pub capability: Option<String>,
    pub tags: BTreeMap<String, String>,
    pub since_unix_ms: Option<u64>,
}

impl ArtifactRecord {
    /// Resolve a dotted predicate path against this record, returning the value
    /// as a string when the path is known. Returns `None` for unknown paths or
    /// when the field is unset; the caller treats `None` as a non-match.
    pub fn lookup_path(&self, path: &str) -> Option<String> {
        if let Some(tag_key) = path.strip_prefix("tags.") {
            return self.tags.get(tag_key).cloned();
        }
        match path {
            "id" => Some(self.id.clone()),
            "run_id" => Some(self.run_id.clone()),
            "name" => Some(self.name.clone()),
            "producer_name" => Some(self.producer_name.clone()),
            "producer_kind" => Some(producer_kind_str(self.producer_kind).to_string()),
            "artifact_type" => Some(artifact_type_str(self.artifact_type).to_string()),
            "step_name" => self.step_name.clone(),
            "content_type" => self.content_type.clone(),
            "version" => Some(self.version.to_string()),
            _ => None,
        }
    }
}

fn producer_kind_str(kind: ProducerKind) -> &'static str {
    match kind {
        ProducerKind::Capability => "capability",
        ProducerKind::Action => "action",
        ProducerKind::Workflow => "workflow",
    }
}

impl RunRecord {
    /// Resolve a dotted predicate path against this record, returning the value
    /// as a string when the path is known. Unknown paths return None which the
    /// caller treats as a non-match. Mirrors ArtifactRecord::lookup_path.
    pub fn lookup_path(&self, path: &str) -> Option<String> {
        if let Some(rest) = path.strip_prefix("inputs.") {
            return self.inputs.get(rest).map(value_to_predicate_string);
        }
        if let Some(rest) = path.strip_prefix("output.") {
            return self
                .output
                .as_ref()
                .and_then(|value| value.get(rest))
                .map(value_to_predicate_string);
        }
        match path {
            "id" => Some(self.id.clone()),
            "kind" => Some(run_kind_str(self.kind).to_string()),
            "status" => Some(run_status_str(self.status).to_string()),
            "mode" => Some(run_mode_str(self.mode).to_string()),
            "target_name" => Some(self.target_name.clone()),
            "error_message" => self.error_message.clone(),
            "source.kind" => Some(run_source_kind_str(&self.source).to_string()),
            "source.workflow_run_id" => match &self.source {
                RunSource::Workflow {
                    workflow_run_id, ..
                } => Some(workflow_run_id.clone()),
                _ => None,
            },
            "source.step_name" => match &self.source {
                RunSource::Workflow { step_name, .. } => Some(step_name.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

fn run_kind_str(kind: RunKind) -> &'static str {
    match kind {
        RunKind::Action => "action",
        RunKind::Workflow => "workflow",
    }
}

fn run_status_str(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Planned => "planned",
        RunStatus::Running => "running",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
    }
}

fn run_mode_str(mode: RunMode) -> &'static str {
    match mode {
        RunMode::PlanOnly => "plan-only",
        RunMode::Execute => "execute",
    }
}

fn run_source_kind_str(source: &RunSource) -> &'static str {
    match source {
        RunSource::Cli => "cli",
        RunSource::Mcp => "mcp",
        RunSource::Workflow { .. } => "workflow",
    }
}

fn value_to_predicate_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

fn artifact_type_str(kind: crate::domain::ArtifactType) -> &'static str {
    match kind {
        crate::domain::ArtifactType::Resource => "resource",
        crate::domain::ArtifactType::File => "file",
    }
}

pub trait DatastoreProvider {
    fn provider_name(&self) -> &str;
    fn put_run(&self, record: &RunRecord) -> Result<()>;
    fn get_run(&self, id: &str) -> Result<Option<RunRecord>>;
    fn list_runs(&self, query: &ListRunsQuery) -> Result<Vec<RunRecord>>;
    fn put_artifact(&self, record: &ArtifactRecord) -> Result<()>;
    fn get_artifact(&self, id: &str) -> Result<Option<ArtifactRecord>>;
    fn list_artifacts(&self, query: &ListArtifactsQuery) -> Result<Vec<ArtifactRecord>>;
}

pub struct FilesystemProvider {
    root: PathBuf,
}

impl FilesystemProvider {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }

    fn artifacts_dir(&self) -> PathBuf {
        self.root.join("artifacts")
    }

    fn run_path(&self, id: &str) -> PathBuf {
        self.runs_dir().join(format!("{id}.json"))
    }

    fn artifact_path(&self, id: &str) -> PathBuf {
        self.artifacts_dir().join(format!("{id}.json"))
    }
}

impl DatastoreProvider for FilesystemProvider {
    fn provider_name(&self) -> &str {
        FILESYSTEM_PROVIDER
    }

    fn put_run(&self, record: &RunRecord) -> Result<()> {
        let path = self.run_path(&record.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(record)?)?;
        Ok(())
    }

    fn get_run(&self, id: &str) -> Result<Option<RunRecord>> {
        let path = self.run_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&raw)?))
    }

    fn list_runs(&self, query: &ListRunsQuery) -> Result<Vec<RunRecord>> {
        let mut records = Vec::new();
        let dir = self.runs_dir();
        if !dir.exists() {
            return Ok(records);
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = fs::read_to_string(&path)?;
            let record: RunRecord = serde_json::from_str(&raw)?;
            if let Some(kind) = query.kind
                && record.kind != kind
            {
                continue;
            }
            if let Some(status) = query.status
                && record.status != status
            {
                continue;
            }
            if let Some(threshold) = query.since_unix_ms
                && record.started_at_unix_ms < threshold
            {
                continue;
            }
            records.push(record);
        }
        records.sort_by(|a, b| b.started_at_unix_ms.cmp(&a.started_at_unix_ms));
        Ok(records)
    }

    fn put_artifact(&self, record: &ArtifactRecord) -> Result<()> {
        let path = self.artifact_path(&record.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(record)?)?;
        Ok(())
    }

    fn get_artifact(&self, id: &str) -> Result<Option<ArtifactRecord>> {
        let path = self.artifact_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&raw)?))
    }

    fn list_artifacts(&self, query: &ListArtifactsQuery) -> Result<Vec<ArtifactRecord>> {
        let mut records = Vec::new();
        let dir = self.artifacts_dir();
        if !dir.exists() {
            return Ok(records);
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = fs::read_to_string(&path)?;
            let record: ArtifactRecord = serde_json::from_str(&raw)?;
            if let Some(run_id) = query.run_id.as_deref()
                && record.run_id != run_id
            {
                continue;
            }
            if let Some(name) = query.name.as_deref()
                && record.name != name
            {
                continue;
            }
            if let Some(capability) = query.capability.as_deref()
                && (record.producer_kind != ProducerKind::Capability
                    || record.producer_name != capability)
            {
                continue;
            }
            if let Some(threshold) = query.since_unix_ms
                && record.created_at_unix_ms < threshold
            {
                continue;
            }
            if !query.tags.is_empty()
                && !query
                    .tags
                    .iter()
                    .all(|(k, v)| record.tags.get(k).is_some_and(|actual| actual == v))
            {
                continue;
            }
            records.push(record);
        }
        records.sort_by(|a, b| b.created_at_unix_ms.cmp(&a.created_at_unix_ms));
        Ok(records)
    }
}

pub fn open_provider(
    repo_root: &Path,
    config: &DatastoreConfig,
) -> Result<Box<dyn DatastoreProvider>> {
    match config.provider.as_str() {
        FILESYSTEM_PROVIDER => {
            let root = resolve_filesystem_datastore_root(repo_root, config);
            Ok(Box::new(FilesystemProvider::new(root)))
        }
        other => bail!(
            "datastore provider '{}' is not built in; only '{}' is available in this slice",
            other,
            FILESYSTEM_PROVIDER
        ),
    }
}

pub fn open_repo_datastore(repo_root: &Path) -> Result<(LoadedRepoConfig, Box<dyn DatastoreProvider>)> {
    let loaded = load_repo_config(repo_root)?;
    let provider = open_provider(repo_root, &loaded.config.datastore)?;
    Ok((loaded, provider))
}
