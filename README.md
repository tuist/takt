# Takt 🔥

Most of the energy around agents right now is going into layers that sit above them: planners, orchestrators, workflow engines, and harnesses. Takt makes a different bet. The agent harness should remain the main interface, and the framework should live underneath it.

That matters because the user is already working through the harness. That is where intent is expressed, progress is reviewed, and taste enters the loop. Takt gives the agent a durable substrate when ad hoc work wants to harden into structure: capabilities it can reuse, actions it can configure for a project, workflows it can run, and artifacts it can inspect afterward.

If you are new to Takt, start with these concepts:

- **Package**: a publishable collection of reusable capabilities.
- **Capability**: a reusable thing a package can do.
- **Action**: a project-specific configuration of a capability.
- **Workflow**: actions wired together into a larger task.
- **Run**: one execution of an action or workflow.
- **Artifact**: the output produced by a run.

## Current Surface

- `takt concepts` to explain the core nouns
- `takt schema` to emit machine-readable schemas
- `takt init` to scaffold a package
- `takt generate action` and `takt generate workflow` to create starter manifests
- `takt validate` to check package and manifest correctness
- `takt run` to plan action and workflow runs
- `takt mcp` to expose the same model through MCP

## Design Principles

Takt is built around a few core principles:

- one shared core for CLI and MCP behavior
- structured output for agent-friendly automation
- thin agent skills that route to executable interfaces instead of duplicating behavior in markdown

## Command Examples

```sh
takt concepts
takt schema all --format json
takt init
takt validate
takt run action <name>
takt mcp
```

## License

MIT
