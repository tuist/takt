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
    let mut files = starter_handler_files(project_root);
    match coding_agent {
        CodingAgent::Codex => files.extend(codex_bootstrap_files(project_root, package_name)),
        CodingAgent::None => {}
    }
    files
}

fn starter_handler_files(project_root: &Path) -> Vec<ScaffoldFile> {
    vec![ScaffoldFile::new(
        project_path(project_root, "handlers/example.mjs"),
        "starter handler",
        include_str!("../templates/bootstrap/handlers/example.mjs").to_string(),
    )]
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

#[cfg(test)]
mod tests {
    use super::{CodingAgent, package_bootstrap_files, package_project_root, render_template};
    use std::path::{Path, PathBuf};

    #[test]
    fn package_project_root_defaults_to_current_directory() {
        assert_eq!(
            package_project_root(Path::new("takt.json")),
            PathBuf::from(".")
        );
    }

    #[test]
    fn package_project_root_uses_manifest_parent_directory() {
        assert_eq!(
            package_project_root(Path::new("packages/example/takt.json")),
            PathBuf::from("packages/example")
        );
    }

    #[test]
    fn package_bootstrap_files_include_only_the_starter_handler_when_coding_agent_is_disabled() {
        let files = package_bootstrap_files(Path::new("."), "@acme/test", CodingAgent::None);
        let paths: Vec<_> = files.iter().map(|file| file.path.as_path()).collect();
        assert_eq!(paths, vec![Path::new("handlers/example.mjs")]);
        assert_eq!(files[0].label, "starter handler");
        assert!(files[0].contents.contains("TAKT_RESULT_PATH"));
    }

    #[test]
    fn package_bootstrap_files_include_starter_handler_and_agents_skills() {
        let files = package_bootstrap_files(Path::new("."), "@acme/test", CodingAgent::Codex);
        let paths: Vec<_> = files.iter().map(|file| file.path.as_path()).collect();

        assert_eq!(files.len(), 6);
        assert_eq!(
            paths,
            vec![
                Path::new("handlers/example.mjs"),
                Path::new("AGENTS.md"),
                Path::new(".agents/skills/takt-getting-started/SKILL.md"),
                Path::new(".agents/skills/takt-package/SKILL.md"),
                Path::new(".agents/skills/takt-action/SKILL.md"),
                Path::new(".agents/skills/takt-workflow/SKILL.md"),
            ]
        );
        assert_eq!(files[1].label, "agent guide");
        assert!(
            files[1]
                .contents
                .contains("This package is named `@acme/test`.")
        );
        assert!(files[1].contents.contains("`takt concepts --format toon`"));
        assert!(files[1].contents.contains("## Getting Started"));
        assert!(
            files[1]
                .contents
                .contains("`.agents/skills/takt-workflow/SKILL.md`")
        );
        assert!(
            files[2]
                .contents
                .contains("Prefer CLI TOON output over prose in this file")
        );
        assert!(files[2].contents.contains("start -> package_inspected"));
        assert!(
            files[4]
                .contents
                .contains("Never write an action manifest from scratch.")
        );
    }

    #[test]
    fn package_bootstrap_files_respect_custom_project_root() {
        let files = package_bootstrap_files(
            Path::new("packages/example"),
            "@acme/test",
            CodingAgent::Codex,
        );

        assert_eq!(
            files[0].path,
            PathBuf::from("packages/example/handlers/example.mjs")
        );
        assert_eq!(files[1].path, PathBuf::from("packages/example/AGENTS.md"));
        assert_eq!(
            files[5].path,
            PathBuf::from("packages/example/.agents/skills/takt-workflow/SKILL.md")
        );
    }

    #[test]
    fn render_template_replaces_named_placeholders() {
        let rendered = render_template(
            "Hello, {{name}}. Welcome to {{place}}.",
            &[("name", "Ada"), ("place", "Takt")],
        );

        assert_eq!(rendered, "Hello, Ada. Welcome to Takt.");
    }
}
