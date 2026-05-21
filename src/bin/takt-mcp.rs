use color_eyre::eyre::Result;
use rmcp::{
    Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct TaktMcpServer {
    tool_router: ToolRouter<Self>,
}

impl TaktMcpServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for TaktMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(
    name = "takt",
    version = "0.1.0",
    instructions = "Use Takt tools for package scaffolding, validation, and run planning. Prefer these tools over scraping CLI output.",
    router = self.tool_router
)]
impl ServerHandler for TaktMcpServer {}

#[tool_router(router = tool_router)]
impl TaktMcpServer {
    #[tool(
        name = "concepts_get",
        description = "Get the canonical Takt object model and runtime rule"
    )]
    async fn concepts_get(&self) -> Result<Json<Value>, String> {
        json_value(takt::core::concepts())
    }

    #[tool(
        name = "schema_get",
        description = "Get JSON Schema for Takt domain objects"
    )]
    async fn schema_get(
        &self,
        Parameters(params): Parameters<SchemaGetParams>,
    ) -> Result<Json<Value>, String> {
        let target = match params.target.as_deref().unwrap_or("all") {
            "all" => takt::core::SchemaTarget::All,
            "package" => takt::core::SchemaTarget::Package,
            "runtime" => takt::core::SchemaTarget::Runtime,
            "capability" => takt::core::SchemaTarget::Capability,
            "action" => takt::core::SchemaTarget::Action,
            "workflow" => takt::core::SchemaTarget::Workflow,
            other => {
                return Err(format!(
                    "invalid schema target '{other}', expected one of all, package, runtime, capability, action, workflow"
                ));
            }
        };

        Ok(Json(takt::core::schema_for_target(target)))
    }

    #[tool(
        name = "repo_init",
        description = "Initialize a Takt package repository and bootstrap agent guidance"
    )]
    async fn repo_init(
        &self,
        Parameters(params): Parameters<RepoInitParams>,
    ) -> Result<Json<Value>, String> {
        json_result(takt::core::init_package(
            params.name,
            params.description,
            params
                .output
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("package.yaml")),
            params.force.unwrap_or(false),
        ))
    }

    #[tool(
        name = "action_generate",
        description = "Generate a starter action manifest"
    )]
    async fn action_generate(
        &self,
        Parameters(params): Parameters<ActionGenerateParams>,
    ) -> Result<Json<Value>, String> {
        json_result(takt::core::generate_action(
            params.name,
            params.capability,
            params.output.map(PathBuf::from),
            params.force.unwrap_or(false),
        ))
    }

    #[tool(
        name = "workflow_generate",
        description = "Generate a starter workflow manifest"
    )]
    async fn workflow_generate(
        &self,
        Parameters(params): Parameters<WorkflowGenerateParams>,
    ) -> Result<Json<Value>, String> {
        json_result(takt::core::generate_workflow(
            params.name,
            params.uses,
            params.output.map(PathBuf::from),
            params.force.unwrap_or(false),
        ))
    }

    #[tool(
        name = "package_validate",
        description = "Validate the package manifest in a Takt repository"
    )]
    async fn package_validate(
        &self,
        Parameters(params): Parameters<RepoScopedParams>,
    ) -> Result<Json<Value>, String> {
        let repo = load_repo(params.repo_dir)?;
        json_value(takt::core::validate_package(&repo))
    }

    #[tool(
        name = "action_validate",
        description = "Validate an action manifest by name or path"
    )]
    async fn action_validate(
        &self,
        Parameters(params): Parameters<ActionSelectorParams>,
    ) -> Result<Json<Value>, String> {
        let repo = load_repo(params.repo_dir)?;
        let action = takt::core::load_action(&repo, &params.selector).map_err(to_string)?;
        json_value(takt::core::validate_action_document(&repo, &action))
    }

    #[tool(
        name = "workflow_validate",
        description = "Validate a workflow manifest by name or path"
    )]
    async fn workflow_validate(
        &self,
        Parameters(params): Parameters<WorkflowSelectorParams>,
    ) -> Result<Json<Value>, String> {
        let repo = load_repo(params.repo_dir)?;
        let workflow = takt::core::load_workflow(&repo, &params.selector).map_err(to_string)?;
        json_value(takt::core::validate_workflow_document(&repo, &workflow))
    }

    #[tool(
        name = "validate_all",
        description = "Validate the package plus all local action and workflow manifests"
    )]
    async fn validate_all(
        &self,
        Parameters(params): Parameters<RepoScopedParams>,
    ) -> Result<Json<Value>, String> {
        let repo = load_repo(params.repo_dir)?;
        json_result(takt::core::validate_all(&repo))
    }

    #[tool(
        name = "action_run_plan",
        description = "Resolve and persist a planned action run without executing it"
    )]
    async fn action_run_plan(
        &self,
        Parameters(params): Parameters<RunPlanParams>,
    ) -> Result<Json<Value>, String> {
        let repo = load_repo(params.repo_dir)?;
        json_result(takt::core::plan_action_run(
            &repo,
            &params.selector,
            params.inputs.unwrap_or_default(),
            params.persist.unwrap_or(true),
        ))
    }

    #[tool(
        name = "workflow_run_plan",
        description = "Resolve and persist a planned workflow run without executing it"
    )]
    async fn workflow_run_plan(
        &self,
        Parameters(params): Parameters<RunPlanParams>,
    ) -> Result<Json<Value>, String> {
        let repo = load_repo(params.repo_dir)?;
        json_result(takt::core::plan_workflow_run(
            &repo,
            &params.selector,
            params.inputs.unwrap_or_default(),
            params.persist.unwrap_or(true),
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SchemaGetParams {
    target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct RepoInitParams {
    name: String,
    description: Option<String>,
    output: Option<String>,
    force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ActionGenerateParams {
    name: String,
    capability: String,
    output: Option<String>,
    force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WorkflowGenerateParams {
    name: String,
    uses: String,
    output: Option<String>,
    force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct RepoScopedParams {
    repo_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ActionSelectorParams {
    selector: String,
    repo_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WorkflowSelectorParams {
    selector: String,
    repo_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct RunPlanParams {
    selector: String,
    repo_dir: Option<String>,
    inputs: Option<BTreeMap<String, Value>>,
    persist: Option<bool>,
}

fn json_result<T>(
    result: std::result::Result<T, impl std::fmt::Display>,
) -> Result<Json<Value>, String>
where
    T: Serialize,
{
    let value = result.map_err(|error| error.to_string())?;
    json_value(value)
}

fn json_value<T>(value: T) -> Result<Json<Value>, String>
where
    T: Serialize,
{
    serde_json::to_value(value).map(Json).map_err(to_string)
}

fn load_repo(repo_dir: Option<String>) -> Result<takt::core::Repository, String> {
    let start = repo_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    takt::core::discover_repository(start).map_err(to_string)
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let service = TaktMcpServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
