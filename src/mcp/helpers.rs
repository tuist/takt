use color_eyre::eyre::Result;
use rmcp::Json;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

pub(super) fn json_result<T>(
    result: std::result::Result<T, impl std::fmt::Display>,
) -> Result<Json<Value>, String>
where
    T: Serialize,
{
    let value = result.map_err(|error| error.to_string())?;
    json_value(value)
}

pub(super) fn json_value<T>(value: T) -> Result<Json<Value>, String>
where
    T: Serialize,
{
    serde_json::to_value(value).map(Json).map_err(to_string)
}

pub(super) fn load_repo(repo_dir: Option<String>) -> Result<crate::core::Repository, String> {
    let start = repo_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    crate::core::discover_repository(start).map_err(to_string)
}

pub(super) fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
