# Takt 🔥

Most of the energy around agents right now is going into layers that sit above them. Better harnesses, better planners, better orchestration, better ways of steering a model through a task. That work matters, but it often leads to a familiar outcome: the framework becomes the product, and the agent becomes a guest inside it.

Takt starts from a different assumption. The agent should remain the main interface. The harness is where the user asks for work, reviews progress, and iterates. The framework should live underneath that experience as an execution layer the agent can inspect, extend, and run.

That is what Takt is for. It gives agents a set of primitives for turning one-off work into something more durable: packaged capabilities, project-local actions, composable workflows, and execution artifacts that can be inspected and reused later.

The current implementation is written in Rust and exposes a CLI and MCP server.

## What Takt Is For

Takt is aimed at teams that want a clearer way to:

- turn repeated agent work into reusable capabilities
- publish those capabilities as packages
- configure those capabilities for a specific project as actions
- compose actions into workflows
- run workflows with explicit runtime and network policy
- keep the user-facing interface in the agent harness while relying on a real execution layer underneath

The canonical object model is:

`package -> capability -> action -> workflow -> run -> artifact`

## Why This Exists

The main idea is to separate reusable registry concepts from project-local configuration:

- packages publish capabilities
- actions configure how a project uses a capability
- workflows orchestrate actions instead of pointing at raw scripts or container images

That gives the system a clearer contract for validation, execution, and agent tooling.

## Runtime Model

Capabilities execute on named runtime profiles. A runtime profile declares the sandbox, pinned OCI image, CPU and memory limits, and network policy for execution. The current direction points toward reviewed, constrained runtimes instead of ad hoc shell scripts.

## Current Surface

The current CLI and MCP surface centers on:

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
