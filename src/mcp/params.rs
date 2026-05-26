use crate::scaffold::CodingAgent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct SchemaGetParams {
    pub target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct PackageInitParams {
    pub name: String,
    pub description: Option<String>,
    pub output: Option<String>,
    pub coding_agent: Option<CodingAgent>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ActionGenerateParams {
    pub name: String,
    pub capability: String,
    pub output: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct WorkflowGenerateParams {
    pub name: String,
    pub uses: String,
    pub output: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct PackageScopedParams {
    pub package_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ActionSelectorParams {
    pub selector: String,
    pub package_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct WorkflowSelectorParams {
    pub selector: String,
    pub package_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ActionRunParams {
    pub selector: String,
    pub package_dir: Option<String>,
    pub inputs: Option<BTreeMap<String, Value>>,
    pub persist: Option<bool>,
    /// If true, validate + resolve only; do not invoke the handler.
    pub plan_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct WorkflowRunParams {
    pub selector: String,
    pub package_dir: Option<String>,
    pub inputs: Option<BTreeMap<String, Value>>,
    pub persist: Option<bool>,
    /// If true, validate + resolve all steps without invoking any handler.
    pub plan_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct RunListParams {
    pub package_dir: Option<String>,
    /// "action" or "workflow"
    pub kind: Option<String>,
    /// "planned" | "running" | "succeeded" | "failed"
    pub status: Option<String>,
    /// Compact duration like 30s, 5m, 2h, 7d
    pub since: Option<String>,
    pub limit: Option<usize>,
    /// Equality predicates over record paths (e.g. "source.kind=workflow"); all ANDed
    pub r#where: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct RunGetParams {
    pub id: String,
    pub package_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ArtifactListParams {
    pub package_dir: Option<String>,
    pub run: Option<String>,
    pub name: Option<String>,
    pub capability: Option<String>,
    /// Required tag values; every entry must match
    pub tags: Option<BTreeMap<String, String>>,
    pub since: Option<String>,
    pub limit: Option<usize>,
    /// Equality predicates over record paths (e.g. "tags.env=prod"); all ANDed
    pub r#where: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ArtifactGetParams {
    pub id: String,
    pub package_dir: Option<String>,
}
