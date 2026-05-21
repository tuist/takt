use color_eyre::eyre::Result;
use rmcp::ErrorData;
use std::path::PathBuf;

pub(super) fn load_repo(repo_dir: Option<String>) -> Result<crate::core::Repository, String> {
    let start = repo_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    crate::core::discover_repository(start).map_err(to_string)
}

pub(super) fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

pub(super) fn tool_error(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}
