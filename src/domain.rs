use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const API_VERSION: &str = "takt.dev/v1alpha1";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PackageManifest {
    pub api_version: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub node: String,
    #[serde(default)]
    pub runtimes: BTreeMap<String, RuntimeProfile>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, CapabilityDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeProfile {
    /// "process" runs the handler as a plain Node subprocess. "microsandbox"
    /// invokes the `msb` CLI to run the handler inside a microVM.
    pub sandbox: String,
    /// OCI image reference used when sandbox=microsandbox (e.g. node:22-alpine
    /// or a digest-pinned reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub network: NetworkPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetworkPolicy {
    /// "disabled" | "allow-all". Maps to `msb --no-network` or `--network-policy allow-all`.
    #[serde(default = "default_network_mode")]
    pub mode: String,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            mode: default_network_mode(),
        }
    }
}

fn default_network_mode() -> String {
    "disabled".to_string()
}

pub const SANDBOX_PROCESS: &str = "process";
pub const SANDBOX_MICROSANDBOX: &str = "microsandbox";
pub const DEFAULT_RUNTIME_NAME: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub handler: HandlerDefinition,
    pub input: SchemaReference,
    pub output: SchemaReference,
    /// Runtime profile name (key into PackageManifest.runtimes). Falls back to
    /// "default" when unset; if no matching profile exists, the handler runs
    /// as a plain Node subprocess with no isolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "PermissionPolicy::is_empty")]
    pub permissions: PermissionPolicy,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifacts: BTreeMap<String, ArtifactDeclaration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactType {
    Resource,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactDeclaration {
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HandlerDefinition {
    pub entrypoint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SchemaReference {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PermissionPolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_paths: Vec<String>,
}

impl PermissionPolicy {
    pub fn is_empty(&self) -> bool {
        self.secret_refs.is_empty() && self.read_paths.is_empty() && self.write_paths.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionDefinition {
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub capability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, SecretBinding>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretBinding {
    pub source: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowDefinition {
    pub api_version: String,
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, Value>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStep {
    pub name: String,
    pub uses: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreach: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_expression: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifacts: BTreeMap<String, WorkflowStepArtifactOverride>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStepArtifactOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<RetentionPolicy>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetentionPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<Lifetime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_latest: Option<u32>,
}

/// How long an artifact should live before garbage collection considers it.
///
/// `Duration` carries a string like `30d` or `1h`; bounded variants describe
/// well-known scopes so handlers and the GC do not have to guess.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Lifetime {
    Ephemeral,
    Job,
    Workflow,
    Infinite,
    Duration { value: String },
}

impl PackageManifest {
    pub fn starter(name: String, description: Option<String>) -> Self {
        let mut capabilities = BTreeMap::new();
        capabilities.insert(
            "example.run".into(),
            CapabilityDefinition {
                description: Some("Example capability scaffold".into()),
                handler: HandlerDefinition {
                    entrypoint: "handlers/example.mjs".into(),
                    argv: vec![],
                },
                input: SchemaReference {
                    path: "schemas/example-input.json".into(),
                    description: Some("Input schema for the example capability".into()),
                },
                output: SchemaReference {
                    path: "schemas/example-output.json".into(),
                    description: Some("Output schema for the example capability".into()),
                },
                runtime: Some(DEFAULT_RUNTIME_NAME.into()),
                permissions: PermissionPolicy::default(),
                artifacts: BTreeMap::new(),
            },
        );

        let mut runtimes = BTreeMap::new();
        runtimes.insert(
            DEFAULT_RUNTIME_NAME.into(),
            RuntimeProfile {
                sandbox: SANDBOX_PROCESS.into(),
                image: None,
                cpus: None,
                memory_mb: None,
                network: NetworkPolicy::default(),
            },
        );

        Self {
            api_version: API_VERSION.into(),
            name,
            version: "0.1.0".into(),
            description,
            node: "22.12.0".into(),
            runtimes,
            capabilities,
        }
    }
}

impl ActionDefinition {
    pub fn starter(name: String, capability: String) -> Self {
        Self {
            api_version: API_VERSION.into(),
            kind: "Action".into(),
            name,
            capability,
            description: Some("Project-local configured action scaffold".into()),
            with: BTreeMap::new(),
            secrets: BTreeMap::new(),
            labels: BTreeMap::new(),
        }
    }
}

impl WorkflowDefinition {
    pub fn starter(name: String, uses: String) -> Self {
        Self {
            api_version: API_VERSION.into(),
            kind: "Workflow".into(),
            name,
            description: Some("Workflow scaffold".into()),
            inputs: BTreeMap::new(),
            steps: vec![WorkflowStep {
                name: "step-1".into(),
                uses,
                with: BTreeMap::new(),
                needs: vec![],
                foreach: None,
                if_expression: None,
                artifacts: BTreeMap::new(),
            }],
        }
    }
}
