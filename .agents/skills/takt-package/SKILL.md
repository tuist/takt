---
name: takt-package
description: >
  Draft Takt package skill. Use when designing package manifests,
  capabilities, runtime profiles, or registry behavior.
---

# Takt Package

Packages are the publishable units in the Fragua registry.

## Responsibilities

- publish capabilities
- define runtime profiles
- declare handler entrypoints
- point at input and output schemas

## Rules

1. Search for an existing package before inventing a new one.
2. A capability must reference a named runtime profile.
3. Runtime profiles must pin Microsandbox OCI images by digest.
4. Capability permissions must be explicit: secrets, read paths, write paths,
   and network mode.

## Recommended Review Flow

1. Check `docs/architecture/takt-foundations.md`.
2. Inspect the current Rust schema with `cargo run -- schema package`.
3. Verify the capability/runtime split is still clean.

## Current Command

Use `cargo run -- package init <name>` to scaffold a package manifest, then
edit the generated YAML.

## Smells

- package names appearing directly in workflow steps
- raw container images referenced by workflows
- one-off script paths standing in for capabilities
