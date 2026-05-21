---
name: takt-package
description: >
  Design or review a Takt package manifest. Use when working on capabilities,
  runtime profiles, handlers, schemas, or registry-facing package structure.
---

# Takt Package

Packages are the publishable units in Takt.
This skill is a routing guide. Treat `takt schema package --format toon` and `takt.json` as the source of truth.

## Responsibilities

- publish capabilities
- define runtime profiles
- declare handler entrypoints
- point at input and output schemas

## Review Flow

1. Read `takt.json`.
2. Run `takt schema package --format toon`.
3. Run `takt validate package --format toon`.
4. Confirm every capability references a named runtime profile.
5. Confirm runtime policy is explicit: image digest, CPU, memory, network, and
   secrets.

## Rules

1. Search the local package before inventing a new capability.
2. Capabilities must reference named runtime profiles.
3. Runtime profiles should pin Microsandbox OCI images by digest.
4. Workflow files must never reference package names or container images
   directly.

## Current Command

Use `takt init <name>` to scaffold a new package, then edit
`takt.json`.

## Smells

- package names appearing directly in workflow steps
- raw container images referenced by workflows
- one-off script paths standing in for capabilities
