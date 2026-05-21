---
name: takt-package
description: >
  Design or review a Takt package manifest. Use when working on capabilities,
  runtime profiles, handlers, schemas, or registry-facing package structure.
---

# Takt Package

Packages are the publishable units in Takt.

## Responsibilities

- publish capabilities
- define runtime profiles
- declare handler entrypoints
- point at input and output schemas

## Review Flow

1. Read `package.yaml`.
2. Run `takt schema package --format json`.
3. Confirm every capability references a named runtime profile.
4. Confirm runtime policy is explicit: image digest, CPU, memory, network, and
   secrets.

## Rules

1. Search the local package before inventing a new capability.
2. Capabilities must reference named runtime profiles.
3. Runtime profiles should pin Microsandbox OCI images by digest.
4. Workflow files must never reference package names or container images
   directly.

## Current Command

Use `takt init <name>` to scaffold a new package repository, then edit
`package.yaml`.

## Smells

- package names appearing directly in workflow steps
- raw container images referenced by workflows
- one-off script paths standing in for capabilities
