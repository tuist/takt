# Project

This repository is a Takt package named `@acme/test`.

## Rules

1. Packages publish capabilities. Actions configure capabilities for this project. Workflows orchestrate actions.
2. Workflows depend on actions, never raw scripts, OCI images, or package names directly.
3. Capabilities execute on named runtime profiles. Pin Microsandbox OCI images by digest and declare network and secret policy explicitly.
4. Search the local package manifest before inventing a new capability or action.
5. If the CLI shape is unclear, inspect it with `takt concepts --format json` and `takt schema all --format json`.

## Skills

- `.agents/skills/takt-getting-started/SKILL.md`
- `.agents/skills/takt-package/SKILL.md`
- `.agents/skills/takt-action/SKILL.md`
- `.agents/skills/takt-workflow/SKILL.md`
