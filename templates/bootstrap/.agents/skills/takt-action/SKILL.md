---
name: takt-action
description: >
  Design or review Takt actions. Use when working on project-local configured
  uses of capabilities, including defaults, secret bindings, labels, and
  runtime selection.
---

# Takt Action

Actions are project-local configured uses of capabilities.
This skill is a routing guide. Treat `takt schema action --format toon` and the action manifest as the source of truth.

## Responsibilities

- bind a capability reference
- provide default inputs
- bind secret sources
- attach labels and ownership metadata
- optionally choose a reviewed runtime profile

## Review Flow

1. Read the relevant file under `actions/`.
2. Run `takt schema action --format toon`.
3. Run `takt validate action <name-or-path> --format toon`.
4. Confirm the action is configuration, not implementation.

## Rules

1. Workflows call actions, not capabilities.
2. Actions should hold project-specific configuration, not distributable code.
3. Runtime overrides should be rare and reviewable.
4. Secret bindings should stay declarative.

## Current Command

Use `takt generate action <name> <capability>` to scaffold an action manifest,
then edit the generated YAML.

## Smells

- actions embedding large script bodies
- actions duplicating package logic
- actions bypassing capability permissions
