---
name: takt-package
description: >
  Design or review a Takt package manifest. Use when working on capabilities,
  the package Node version, handlers, schemas, or registry-facing package
  structure.
---

# Takt Package

Packages are the publishable units in Takt.
This skill is a routing guide. Treat `takt schema package --format toon` and `takt.json` as the source of truth.

## Critical Rules

- Never put workflow orchestration in `takt.json`. Packages publish capabilities and pin one exact Node version.
- Never let workflow concerns leak into capability definitions through raw step scripts or inline OCI images.
- If unsure about available fields, run `takt schema package --format toon` instead of guessing.
- Validate after every meaningful package edit with `takt validate package --format toon`.

## Quick Reference

| Task | Command |
| --- | --- |
| Get package schema | `takt schema package --format toon` |
| Validate package | `takt validate package --format toon` |
| Inspect concepts | `takt concepts --format toon` |
| Scaffold package | `takt init <name>` |

## Responsibilities

- publish capabilities
- pin an exact Node version
- declare handler entrypoints
- point at input and output schemas

## Review Flow

1. Read `takt.json`.
2. Run `takt schema package --format toon`.
3. Run `takt validate package --format toon`.
4. Confirm the package pins an exact Node version.
5. Confirm each capability defines a handler plus input and output schemas.

## Rules

1. Search the local package before inventing a new capability.
2. The package Node version is the execution contract for every capability.
3. Capabilities should define handlers and schemas, not execution-policy knobs.
4. Workflow files must never reference package names or container images
   directly.
5. Capability changes should preserve the distinction between reusable
   interface and package-local action configuration.

## Current Command

Use `takt init <name>` to scaffold a new package, then edit
`takt.json`.

## Smells

- package names appearing directly in workflow steps
- raw container images referenced by workflows
- one-off script paths standing in for capabilities
