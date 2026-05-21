use crate::mcp::helpers::{json_result, json_value, load_repo};
use crate::mcp::params::{
    ActionGenerateParams, ActionSelectorParams, RepoInitParams, RepoScopedParams, RunPlanParams,
    SchemaGetParams, WorkflowGenerateParams, WorkflowSelectorParams,
};
use crate::scaffold::CodingAgent;
use color_eyre::eyre::Result;
use rmcp::{
    Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde_json::Value;
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
        json_value(crate::core::concepts())
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
            "all" => crate::core::SchemaTarget::All,
            "package" => crate::core::SchemaTarget::Package,
            "runtime" => crate::core::SchemaTarget::Runtime,
            "capability" => crate::core::SchemaTarget::Capability,
            "action" => crate::core::SchemaTarget::Action,
            "workflow" => crate::core::SchemaTarget::Workflow,
            other => {
                return Err(format!(
                    "invalid schema target '{other}', expected one of all, package, runtime, capability, action, workflow"
                ));
            }
        };

        Ok(Json(crate::core::schema_for_target(target)))
    }

    #[tool(
        name = "repo_init",
        description = "Initialize a Takt package repository and optionally bootstrap coding-agent guidance"
    )]
    async fn repo_init(
        &self,
        Parameters(params): Parameters<RepoInitParams>,
    ) -> Result<Json<Value>, String> {
        json_result(crate::core::init_package(
            params.name,
            params.description,
            params
                .output
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("package.yaml")),
            params.force.unwrap_or(false),
            params.coding_agent.unwrap_or(CodingAgent::Codex),
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
        json_result(crate::core::generate_action(
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
        json_result(crate::core::generate_workflow(
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
        json_value(crate::core::validate_package(&repo))
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
        let action = crate::core::load_action(&repo, &params.selector)
            .map_err(crate::mcp::helpers::to_string)?;
        json_value(crate::core::validate_action_document(&repo, &action))
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
        let workflow = crate::core::load_workflow(&repo, &params.selector)
            .map_err(crate::mcp::helpers::to_string)?;
        json_value(crate::core::validate_workflow_document(&repo, &workflow))
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
        json_result(crate::core::validate_all(&repo))
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
        json_result(crate::core::plan_action_run(
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
        json_result(crate::core::plan_workflow_run(
            &repo,
            &params.selector,
            params.inputs.unwrap_or_default(),
            params.persist.unwrap_or(true),
        ))
    }
}

pub async fn serve_stdio() -> Result<()> {
    let service = TaktMcpServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
