---
name: takt-workflow
description: >
  Design or review Takt workflows. Use when working on orchestration,
  dependencies, and step wiring across actions.
---

# Takt Workflow

Workflows compose actions into repeatable operations.
This skill is a routing guide. Treat `takt schema workflow --format toon` and workflow manifests as the source of truth.

## Responsibilities

- define ordered or dependency-based execution
- wire step inputs
- capture conditions and fan-out
- produce runs and artifacts

## Review Flow

1. Read the relevant file under `workflows/`.
2. Run `takt schema workflow --format toon`.
3. Run `takt validate workflow <name-or-path> --format toon`.
4. Check that every step uses an action reference.

## Rules

1. Workflows depend on actions only.
2. Steps should declare dependencies explicitly.
3. Workflow data flow should prefer structured inputs and artifacts over
   implicit environment mutation.
4. Runtime concerns belong to capabilities and actions, not workflow steps.

## Current Command

Use `takt generate workflow <name> --uses <action>` to scaffold a workflow
manifest, then edit the generated YAML.

## Smells

- workflow steps calling package names directly
- step-level OCI image declarations
- orchestration logic hidden inside action handlers
