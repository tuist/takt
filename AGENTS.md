# Project

This repository is building Takt, a package-driven workflow system for agent
operations.

## Rules

1. Packages publish capabilities. Projects create actions from capabilities.
   Workflows orchestrate actions.
2. Workflows depend on actions, never raw scripts, OCI images, or package names
   directly.
3. Runtime profiles are reviewed infrastructure. Pin Microsandbox OCI images by
   digest and declare network and secret policy explicitly.
4. Search for an existing package or capability before inventing a new one.
5. If the CLI shape is unclear, inspect the current prototype with
   `cargo run -- schema all`.

## Skills

- `.agents/skills/takt-getting-started/SKILL.md`
- `.agents/skills/takt-package/SKILL.md`
- `.agents/skills/takt-action/SKILL.md`
- `.agents/skills/takt-workflow/SKILL.md`
