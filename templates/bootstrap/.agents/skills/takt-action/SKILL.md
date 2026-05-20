---
name: takt-action
description: >
  Design or review Takt actions. Use when working on project-local configured
  uses of capabilities, including defaults, secret bindings, labels, and
  runtime selection.
---

# Takt Action

Actions are project-local configured uses of capabilities.

## Responsibilities

- bind a capability reference
- provide default inputs
- bind secret sources
- attach labels and ownership metadata
- optionally choose a reviewed runtime profile

## Review Flow

1. Read the relevant file under `actions/`.
2. Run `takt schema action`.
3. Confirm the action is configuration, not implementation.

## Rules

1. Workflows call actions, not capabilities.
2. Actions should hold project-specific configuration, not distributable code.
3. Runtime overrides should be rare and reviewable.
4. Secret bindings should stay declarative.

## Current Command

Use `takt action init <name> <capability>` to scaffold an action manifest, then
edit the generated YAML.

## Smells

- actions embedding large script bodies
- actions duplicating package logic
- actions bypassing capability permissions
