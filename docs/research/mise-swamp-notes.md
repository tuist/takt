# Mise and Swamp Notes

## Mise

Reference repo: `https://github.com/jdx/mise`

The useful patterns for Takt are:

- `clap` derive for the command tree
- a dedicated logger that formats human output separately from file logging
- small style helpers for stdout and stderr coloring
- table helpers built on `tabled` and `comfy-table`
- a machine-readable command surface via generated usage metadata

Specific files inspected:

- `src/cli/mod.rs`
- `src/cli/run.rs`
- `src/cli/usage.rs`
- `src/logger.rs`
- `src/ui/style.rs`
- `src/ui/table.rs`
- `src/task/task_output.rs`

## Swamp

Installed version on May 20, 2026:

- `swamp 20260520.085517.0-sha.a85f376a`

The useful patterns for Takt are:

- `swamp repo init --tool codex` generates both `AGENTS.md` rules and a local
  `.agents/skills/` bundle
- each skill is a procedural guide, not a marketing description
- skill files include exact command patterns, verification gates, and failure
  recovery branches
- the CLI exposes a machine-readable command tree with `swamp help ...`

Specific files inspected in a scratch repo:

- `AGENTS.md`
- `.agents/skills/swamp-getting-started/SKILL.md`
- `.agents/skills/swamp-model/SKILL.md`
- `.agents/skills/swamp-workflow/SKILL.md`
- `.agents/skills/swamp-extension/SKILL.md`
- `.agents/skills/swamp-repo/SKILL.md`

## Takt Takeaways

- Takt should ship local skills with the repo, not just product docs
- those skills should teach package, capability, action, and workflow handling
- Takt should expose machine-readable schemas early
- runtime profiles should be first-class because they are the safe execution
  boundary between capabilities and workflows
