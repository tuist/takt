use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct ScaffoldFile {
    pub label: String,
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CodingAgent {
    Codex,
    None,
}

impl ScaffoldFile {
    pub fn new(path: PathBuf, label: impl Into<String>, contents: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            path,
            contents: contents.into(),
        }
    }
}

pub fn package_project_root(output: &Path) -> PathBuf {
    output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn package_bootstrap_files(
    project_root: &Path,
    package_name: &str,
    coding_agent: CodingAgent,
) -> Vec<ScaffoldFile> {
    match coding_agent {
        CodingAgent::Codex => codex_bootstrap_files(project_root, package_name),
        CodingAgent::None => Vec::new(),
    }
}

fn codex_bootstrap_files(project_root: &Path, package_name: &str) -> Vec<ScaffoldFile> {
    vec![
        ScaffoldFile::new(
            project_path(project_root, "AGENTS.md"),
            "agent guide",
            render_template(
                include_str!("../templates/bootstrap/AGENTS.md.tmpl"),
                &[("package_name", package_name)],
            ),
        ),
        ScaffoldFile::new(
            project_path(project_root, ".agents/skills/takt-getting-started/SKILL.md"),
            "skill",
            include_str!("../templates/bootstrap/.agents/skills/takt-getting-started/SKILL.md"),
        ),
        ScaffoldFile::new(
            project_path(project_root, ".agents/skills/takt-package/SKILL.md"),
            "skill",
            include_str!("../templates/bootstrap/.agents/skills/takt-package/SKILL.md"),
        ),
        ScaffoldFile::new(
            project_path(project_root, ".agents/skills/takt-action/SKILL.md"),
            "skill",
            include_str!("../templates/bootstrap/.agents/skills/takt-action/SKILL.md"),
        ),
        ScaffoldFile::new(
            project_path(project_root, ".agents/skills/takt-workflow/SKILL.md"),
            "skill",
            include_str!("../templates/bootstrap/.agents/skills/takt-workflow/SKILL.md"),
        ),
    ]
}

fn project_path(project_root: &Path, relative_path: &str) -> PathBuf {
    if project_root == Path::new(".") {
        PathBuf::from(relative_path)
    } else {
        project_root.join(relative_path)
    }
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut rendered = template.to_string();

    for (key, value) in replacements {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }

    rendered
}
