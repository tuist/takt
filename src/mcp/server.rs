use crate::mcp::helpers::{load_package, tool_error};
use crate::mcp::output::SchemaGetOutput;
use crate::mcp::params::{
    ActionGenerateParams, ActionRunParams, ActionSelectorParams, ArtifactGetParams,
    ArtifactListParams, PackageInitParams, PackageScopedParams, RunGetParams, RunListParams,
    SchemaGetParams, WorkflowGenerateParams, WorkflowRunParams, WorkflowSelectorParams,
};
use crate::scaffold::CodingAgent;
use color_eyre::eyre::Result;
use rmcp::{
    ErrorData, Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(super) struct TaktMcpServer {
    tool_router: ToolRouter<Self>,
}

impl TaktMcpServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

pub(super) fn new_server() -> TaktMcpServer {
    TaktMcpServer::new()
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
        description = "Get the canonical Takt object model and execution rule"
    )]
    async fn concepts_get(&self) -> Result<Json<crate::core::ConceptsOutput>, ErrorData> {
        Ok(Json(crate::core::concepts()))
    }

    #[tool(
        name = "schema_get",
        description = "Get JSON Schema for Takt domain objects"
    )]
    async fn schema_get(
        &self,
        Parameters(params): Parameters<SchemaGetParams>,
    ) -> Result<Json<SchemaGetOutput>, ErrorData> {
        let target_name = params.target.unwrap_or_else(|| "all".to_string());
        let target = match target_name.as_str() {
            "all" => crate::core::SchemaTarget::All,
            "package" => crate::core::SchemaTarget::Package,
            "capability" => crate::core::SchemaTarget::Capability,
            "action" => crate::core::SchemaTarget::Action,
            "workflow" => crate::core::SchemaTarget::Workflow,
            "run" => crate::core::SchemaTarget::Run,
            "artifact" => crate::core::SchemaTarget::Artifact,
            "config" => crate::core::SchemaTarget::Config,
            other => {
                return Err(ErrorData::invalid_params(
                    format!(
                        "invalid schema target '{other}', expected one of all, package, capability, action, workflow, run, artifact, config"
                    ),
                    None,
                ));
            }
        };

        Ok(Json(SchemaGetOutput {
            target: target_name,
            schema: crate::core::schema_for_target(target),
        }))
    }

    #[tool(
        name = "package_init",
        description = "Initialize a Takt package and optionally bootstrap coding-agent guidance"
    )]
    async fn package_init(
        &self,
        Parameters(params): Parameters<PackageInitParams>,
    ) -> Result<Json<crate::core::InitOutput>, ErrorData> {
        crate::core::init_package(
            params.name,
            params.description,
            params
                .output
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(crate::core::ROOT_MANIFEST_FILENAME)),
            params.force.unwrap_or(false),
            params.coding_agent.unwrap_or(CodingAgent::Codex),
        )
        .map(Json)
        .map_err(tool_error)
    }

    #[tool(
        name = "action_generate",
        description = "Generate a starter action manifest"
    )]
    async fn action_generate(
        &self,
        Parameters(params): Parameters<ActionGenerateParams>,
    ) -> Result<Json<crate::core::ActionGenerateOutput>, ErrorData> {
        crate::core::generate_action(
            params.name,
            params.capability,
            params.output.map(PathBuf::from),
            params.force.unwrap_or(false),
        )
        .map(Json)
        .map_err(tool_error)
    }

    #[tool(
        name = "workflow_generate",
        description = "Generate a starter workflow manifest"
    )]
    async fn workflow_generate(
        &self,
        Parameters(params): Parameters<WorkflowGenerateParams>,
    ) -> Result<Json<crate::core::WorkflowGenerateOutput>, ErrorData> {
        crate::core::generate_workflow(
            params.name,
            params.uses,
            params.output.map(PathBuf::from),
            params.force.unwrap_or(false),
        )
        .map(Json)
        .map_err(tool_error)
    }

    #[tool(
        name = "package_validate",
        description = "Validate the root package manifest"
    )]
    async fn package_validate(
        &self,
        Parameters(params): Parameters<PackageScopedParams>,
    ) -> Result<Json<crate::core::ValidationReport>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        Ok(Json(crate::core::validate_package(&repo)))
    }

    #[tool(
        name = "action_validate",
        description = "Validate an action manifest by name or path"
    )]
    async fn action_validate(
        &self,
        Parameters(params): Parameters<ActionSelectorParams>,
    ) -> Result<Json<crate::core::ValidationReport>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        let action = crate::core::load_action(&repo, &params.selector).map_err(tool_error)?;
        Ok(Json(crate::core::validate_action_document(&repo, &action)))
    }

    #[tool(
        name = "workflow_validate",
        description = "Validate a workflow manifest by name or path"
    )]
    async fn workflow_validate(
        &self,
        Parameters(params): Parameters<WorkflowSelectorParams>,
    ) -> Result<Json<crate::core::ValidationReport>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        let workflow = crate::core::load_workflow(&repo, &params.selector).map_err(tool_error)?;
        Ok(Json(crate::core::validate_workflow_document(
            &repo, &workflow,
        )))
    }

    #[tool(
        name = "validate_all",
        description = "Validate the package plus all local action and workflow manifests"
    )]
    async fn validate_all(
        &self,
        Parameters(params): Parameters<PackageScopedParams>,
    ) -> Result<Json<crate::core::ValidationSummary>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        crate::core::validate_all(&repo)
            .map(Json)
            .map_err(tool_error)
    }

    #[tool(
        name = "action_run",
        description = "Invoke an action's capability handler. Persists a Succeeded or Failed run record plus any emitted artifacts. Pass plan_only=true to validate + resolve without invoking the handler."
    )]
    async fn action_run(
        &self,
        Parameters(params): Parameters<ActionRunParams>,
    ) -> Result<Json<crate::core::ActionRunOutput>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        let inputs = params.inputs.unwrap_or_default();
        let persist = params.persist.unwrap_or(true);
        if params.plan_only.unwrap_or(false) {
            crate::core::plan_action_run(
                &repo,
                &params.selector,
                inputs,
                persist,
                crate::datastore::RunSource::Mcp,
            )
        } else {
            crate::core::execute_action_run(
                &repo,
                &params.selector,
                inputs,
                persist,
                crate::datastore::RunSource::Mcp,
            )
        }
        .map(Json)
        .map_err(tool_error)
    }

    #[tool(
        name = "workflow_run",
        description = "Execute a workflow by running each step in topological order. Pass plan_only=true to validate + resolve without invoking any handler."
    )]
    async fn workflow_run(
        &self,
        Parameters(params): Parameters<WorkflowRunParams>,
    ) -> Result<Json<crate::core::WorkflowRunOutput>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        let inputs = params.inputs.unwrap_or_default();
        let persist = params.persist.unwrap_or(true);
        if params.plan_only.unwrap_or(false) {
            crate::core::plan_workflow_run(
                &repo,
                &params.selector,
                inputs,
                persist,
                crate::datastore::RunSource::Mcp,
            )
        } else {
            crate::core::execute_workflow_run(
                &repo,
                &params.selector,
                inputs,
                persist,
                crate::datastore::RunSource::Mcp,
            )
        }
        .map(Json)
        .map_err(tool_error)
    }

    #[tool(
        name = "run_list",
        description = "List persisted runs from the datastore. Filter by kind, status, age, and limit."
    )]
    async fn run_list(
        &self,
        Parameters(params): Parameters<RunListParams>,
    ) -> Result<Json<crate::query::ListEnvelope<crate::datastore::RunRecord>>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        let kind = params
            .kind
            .as_deref()
            .map(parse_run_kind)
            .transpose()
            .map_err(invalid_params)?;
        let status = params
            .status
            .as_deref()
            .map(parse_run_status)
            .transpose()
            .map_err(invalid_params)?;
        let input = crate::core::RunListInput {
            kind,
            status,
            since: params.since,
            limit: params.limit,
            predicates: params.r#where.unwrap_or_default(),
        };
        crate::core::run_list_envelope(&repo, &input)
            .map(Json)
            .map_err(tool_error)
    }

    #[tool(
        name = "run_get",
        description = "Get a single persisted run record by id"
    )]
    async fn run_get(
        &self,
        Parameters(params): Parameters<RunGetParams>,
    ) -> Result<Json<crate::datastore::RunRecord>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        match crate::core::get_run(&repo, &params.id).map_err(tool_error)? {
            Some(run) => Ok(Json(run)),
            None => Err(ErrorData::invalid_params(
                format!("run '{}' was not found in the datastore", params.id),
                None,
            )),
        }
    }

    #[tool(
        name = "artifact_list",
        description = "List artifacts persisted in the datastore. Filter by run id, name, capability, tags, age, and equality predicates over record paths (e.g. tags.env=prod)."
    )]
    async fn artifact_list(
        &self,
        Parameters(params): Parameters<ArtifactListParams>,
    ) -> Result<Json<crate::query::ListEnvelope<crate::datastore::ArtifactRecord>>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        let input = crate::core::ArtifactListInput {
            run: params.run,
            name: params.name,
            capability: params.capability,
            tags: params.tags.unwrap_or_default(),
            since: params.since,
            limit: params.limit,
            predicates: params.r#where.unwrap_or_default(),
        };
        crate::core::artifact_list_envelope(&repo, &input)
            .map(Json)
            .map_err(tool_error)
    }

    #[tool(
        name = "artifact_get",
        description = "Get a single artifact record by id"
    )]
    async fn artifact_get(
        &self,
        Parameters(params): Parameters<ArtifactGetParams>,
    ) -> Result<Json<crate::datastore::ArtifactRecord>, ErrorData> {
        let repo = load_package(params.package_dir).map_err(tool_error)?;
        match crate::core::get_artifact(&repo, &params.id).map_err(tool_error)? {
            Some(artifact) => Ok(Json(artifact)),
            None => Err(ErrorData::invalid_params(
                format!("artifact '{}' was not found in the datastore", params.id),
                None,
            )),
        }
    }
}

fn invalid_params(message: String) -> ErrorData {
    ErrorData::invalid_params(message, None)
}

fn parse_run_kind(value: &str) -> Result<crate::datastore::RunKind, String> {
    match value {
        "action" => Ok(crate::datastore::RunKind::Action),
        "workflow" => Ok(crate::datastore::RunKind::Workflow),
        other => Err(format!(
            "invalid run kind '{other}', expected 'action' or 'workflow'"
        )),
    }
}

fn parse_run_status(value: &str) -> Result<crate::datastore::RunStatus, String> {
    match value {
        "planned" => Ok(crate::datastore::RunStatus::Planned),
        "running" => Ok(crate::datastore::RunStatus::Running),
        "succeeded" => Ok(crate::datastore::RunStatus::Succeeded),
        "failed" => Ok(crate::datastore::RunStatus::Failed),
        other => Err(format!(
            "invalid run status '{other}', expected planned|running|succeeded|failed"
        )),
    }
}

pub async fn serve_stdio() -> Result<()> {
    let service = new_server().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
