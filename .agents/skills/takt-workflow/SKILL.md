---
name: takt-workflow
description: >
  Draft Takt workflow skill. Use when designing orchestration, dependencies,
  and step wiring across actions.
---

# Takt Workflow

Workflows compose actions into repeatable operations.

## Responsibilities

- define ordered or dependency-based execution
- wire step inputs
- capture conditions and fan-out
- produce runs and artifacts

## Rules

1. Workflows depend on actions only.
2. Steps should declare dependencies explicitly.
3. Workflow data flow should prefer structured inputs and artifacts over
   implicit environment mutation.
4. Runtime concerns belong to capabilities and actions, not workflow steps.

## Review Flow

1. Read `docs/architecture/takt-foundations.md`.
2. Inspect `cargo run -- schema workflow`.
3. Check that every step uses an action reference.

## Current Command

Use `cargo run -- workflow init <name> --uses <action>` to scaffold a workflow
manifest, then edit the generated YAML.

## Smells

- workflow steps calling package names directly
- step-level OCI image declarations
- orchestration logic hidden inside action handlers
