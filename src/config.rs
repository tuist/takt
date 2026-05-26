use crate::domain::{API_VERSION, RuntimeProfile};
use color_eyre::eyre::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const REPO_CONFIG_RELATIVE_PATH: &str = ".takt/config.json";
pub const DEFAULT_DATASTORE_RELATIVE_ROOT: &str = ".takt/datastore";
pub const FILESYSTEM_PROVIDER: &str = "filesystem";
pub const DEFAULT_STORAGE_RUNTIME: &str = "storage";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoConfig {
    #[serde(default = "default_api_version")]
    pub api_version: String,
    #[serde(default)]
    pub datastore: DatastoreConfig,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub runtimes: BTreeMap<String, RuntimeProfile>,
}

fn default_api_version() -> String {
    API_VERSION.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DatastoreConfig {
    pub provider: String,
    #[serde(default = "default_storage_runtime")]
    pub runtime: String,
    #[serde(default)]
    pub config: Value,
}

fn default_storage_runtime() -> String {
    DEFAULT_STORAGE_RUNTIME.to_string()
}

impl Default for DatastoreConfig {
    fn default() -> Self {
        Self {
            provider: FILESYSTEM_PROVIDER.into(),
            runtime: DEFAULT_STORAGE_RUNTIME.into(),
            config: json!({ "root": DEFAULT_DATASTORE_RELATIVE_ROOT }),
        }
    }
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.into(),
            datastore: DatastoreConfig::default(),
            runtimes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedRepoConfig {
    pub config: RepoConfig,
    pub source_path: Option<PathBuf>,
}

pub fn config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(REPO_CONFIG_RELATIVE_PATH)
}

pub fn load_repo_config(repo_root: &Path) -> Result<LoadedRepoConfig> {
    let path = config_path(repo_root);
    if !path.exists() {
        return Ok(LoadedRepoConfig {
            config: RepoConfig::default(),
            source_path: None,
        });
    }

    let raw = fs::read_to_string(&path)?;
    let config: RepoConfig = serde_json::from_str(&raw)?;
    Ok(LoadedRepoConfig {
        config,
        source_path: Some(path),
    })
}

pub fn resolve_filesystem_datastore_root(repo_root: &Path, config: &DatastoreConfig) -> PathBuf {
    let configured = config
        .config
        .get("root")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_DATASTORE_RELATIVE_ROOT);
    let candidate = PathBuf::from(configured);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    }
}
