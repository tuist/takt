use std::path::{Path, PathBuf};

pub struct ScaffoldFile {
    pub label: String,
    pub path: PathBuf,
    pub contents: String,
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

pub fn package_bootstrap_files(project_root: &Path, package_name: &str) -> Vec<ScaffoldFile> {
    vec![
        ScaffoldFile::new(
            project_path(project_root, "AGENTS.md"),
            "agent guide",
            package_agents(package_name),
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

fn package_agents(package_name: &str) -> String {
    format!(
        "# Project\n\n\
This repository is a Takt package named `{package_name}`.\n\n\
## Rules\n\n\
1. Packages publish capabilities. Actions configure capabilities for this project. Workflows orchestrate actions.\n\
2. Workflows depend on actions, never raw scripts, OCI images, or package names directly.\n\
3. Capabilities execute on named runtime profiles. Pin Microsandbox OCI images by digest and declare network and secret policy explicitly.\n\
4. Search the local package manifest before inventing a new capability or action.\n\
5. If the CLI shape is unclear, inspect it with `takt concepts --format json` and `takt schema all --format json`.\n\
6. Treat CLI JSON output as authoritative. Skills should route to commands, not duplicate command behavior.\n\n\
## Skills\n\n\
- `.agents/skills/takt-getting-started/SKILL.md`\n\
- `.agents/skills/takt-package/SKILL.md`\n\
- `.agents/skills/takt-action/SKILL.md`\n\
- `.agents/skills/takt-workflow/SKILL.md`\n"
    )
}
