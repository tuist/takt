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
pub(super) struct RepoInitParams {
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
pub(super) struct RepoScopedParams {
    pub repo_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ActionSelectorParams {
    pub selector: String,
    pub repo_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct WorkflowSelectorParams {
    pub selector: String,
    pub repo_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct RunPlanParams {
    pub selector: String,
    pub repo_dir: Option<String>,
    pub inputs: Option<BTreeMap<String, Value>>,
    pub persist: Option<bool>,
}
