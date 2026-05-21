use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub(super) struct SchemaGetOutput {
    pub target: String,
    pub schema: Value,
}
