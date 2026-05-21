# Takt Foundations

Takt's core object model is:

`package -> capability -> action -> workflow -> run -> artifact`

## Canonical Meanings

- `Package`: the publishable unit in the registry.
- `Capability`: a reusable interface exported by a package.
- `Action`: a project-local configured use of a capability.
- `Workflow`: a graph that composes actions.
- `Run`: one execution of an action or workflow.
- `Artifact`: persisted output from a run.

## Why This Split

Swamp's `model type` versus `model definition` split is powerful but easy to
misread, especially because "model" already means "LLM" to many users.

Takt should keep the execution value while using more legible nouns:

- packages publish capabilities
- projects create actions from capabilities
- workflows orchestrate actions

This keeps the registry surface separate from project configuration.

## Runtime Model

Capabilities execute on named runtime profiles. A runtime profile is reviewed
infrastructure and should declare:

- sandbox implementation, currently `microsandbox`
- OCI image pinned by digest
- CPU and memory limits
- network mode plus allow list

Workflows should never point at raw images or scripts directly. They call
actions. Actions resolve to capabilities. Capabilities resolve to runtime
profiles.

## Microsandbox Direction

Microsandbox is the leading runtime candidate because it supports:

- rootless microVM execution
- OCI images from standard registries
- host-controlled network policy
- secret handling that keeps real secrets out of the guest

That makes it a better fit than a single Deno runtime if packages need Ruby,
Python, Bash, or language-specific toolchains.

## CLI Direction

The current Rust prototype focuses on two foundations:

- `takt concepts` for a stable glossary of Takt's core nouns
- `takt schema` for inspectable domain schemas
- `takt init`, `takt generate action`, and `takt generate workflow` for
  starter manifests

Unlike Swamp, which does not expose a dedicated concepts command, Takt uses
`takt concepts` as an explicit onboarding surface for both humans and agents.

Every command should support `--format text|json` so agents can request
structured output without scraping human-oriented tables or status lines.

`takt init` should also bootstrap project-local agent guidance the way
`swamp repo init --tool codex` does: an `AGENTS.md` plus `.agents/skills/`
files that teach an agent how to interact with that initialized package.

The schema command exists because agent-facing tooling should be inspectable.
Swamp does this with `swamp help ...`, and `mise` does something similar with
its generated usage specification.
