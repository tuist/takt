use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const API_VERSION: &str = "takt.dev/v1alpha1";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PackageManifest {
    pub api_version: String,
    pub kind: String,
    pub package: PackageMetadata,
    #[serde(default)]
    pub runtimes: BTreeMap<String, RuntimeProfile>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, CapabilityDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeProfile {
    pub sandbox: SandboxKind,
    pub image: String,
    pub cpus: u8,
    pub memory_mb: u16,
    #[serde(default)]
    pub network: NetworkPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxKind {
    Microsandbox,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct NetworkPolicy {
    pub mode: NetworkMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkMode {
    #[default]
    Disabled,
    AllowList,
    PublicOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub runtime: String,
    pub handler: HandlerDefinition,
    pub input: SchemaReference,
    pub output: SchemaReference,
    #[serde(default, skip_serializing_if = "PermissionPolicy::is_empty")]
    pub permissions: PermissionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HandlerDefinition {
    pub language: HandlerLanguage,
    pub entrypoint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HandlerLanguage {
    Bash,
    #[serde(rename = "javascript")]
    JavaScript,
    #[serde(rename = "typescript")]
    TypeScript,
    Python,
    Ruby,
    Rust,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
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
}

impl PackageManifest {
    pub fn starter(name: String, description: Option<String>) -> Self {
        let mut runtimes = BTreeMap::new();
        runtimes.insert(
            "default".into(),
            RuntimeProfile {
                sandbox: SandboxKind::Microsandbox,
                image: "ghcr.io/example/takt-runtime@sha256:replace-me".into(),
                cpus: 1,
                memory_mb: 512,
                network: NetworkPolicy::default(),
            },
        );

        let mut capabilities = BTreeMap::new();
        capabilities.insert(
            "example.run".into(),
            CapabilityDefinition {
                description: Some("Example capability scaffold".into()),
                runtime: "default".into(),
                handler: HandlerDefinition {
                    language: HandlerLanguage::TypeScript,
                    entrypoint: "handlers/example.ts".into(),
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
                permissions: PermissionPolicy::default(),
            },
        );

        Self {
            api_version: API_VERSION.into(),
            kind: "Package".into(),
            package: PackageMetadata {
                name,
                version: "0.1.0".into(),
                description,
            },
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
            runtime: None,
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
            }],
        }
    }
}
