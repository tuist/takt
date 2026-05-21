---
name: takt-workflow
description: >
  Design or review Takt workflows. Use when working on orchestration,
  dependencies, and step wiring across actions.
---

# Takt Workflow

Workflows compose actions into repeatable operations.
This skill is a routing guide. Treat `takt schema workflow --format toon` and workflow manifests as the source of truth.

## Critical Rules

- Never write a workflow manifest from scratch. Always run `takt generate workflow <name> --uses <action>` first, then edit the generated JSON.
- Never let a workflow reference capabilities, packages, raw scripts, or OCI images directly. Workflows use actions only.
- Always validate after edits with `takt validate workflow <name-or-path> --format toon`.
- If execution behavior matters, inspect the plan with `takt run workflow <name> --format toon`.

## Quick Reference

| Task | Command |
| --- | --- |
| Get workflow schema | `takt schema workflow --format toon` |
| Generate workflow | `takt generate workflow <name> --uses <action>` |
| Validate workflow | `takt validate workflow <name-or-path> --format toon` |
| Validate all manifests | `takt validate all --format toon` |
| Plan workflow run | `takt run workflow <name> --format toon` |

## Responsibilities

- define ordered or dependency-based execution
- wire step inputs
- capture conditions and fan-out
- produce runs and artifacts

## Review Flow

1. If creating a new workflow, scaffold it first with `takt generate workflow <name> --uses <action>`.
2. Read the relevant file under `workflows/`.
3. Run `takt schema workflow --format toon`.
4. Run `takt validate workflow <name-or-path> --format toon`.
5. Check that every step uses an action reference.

## Rules

1. Workflows depend on actions only.
2. Steps should declare dependencies explicitly.
3. Workflow data flow should prefer structured inputs and artifacts over
   implicit environment mutation.
4. Runtime concerns belong to capabilities and actions, not workflow steps.
5. When a workflow changes behavior, re-check the actions it references before adding new steps.

## Current Command

Use `takt generate workflow <name> --uses <action>` to scaffold a workflow
manifest, then edit the generated JSON.

## Smells

- workflow steps calling package names directly
- step-level OCI image declarations
- orchestration logic hidden inside action handlers
