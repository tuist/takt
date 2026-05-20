---
name: takt-action
description: >
  Draft Takt action skill. Use when designing project-local configured uses
  of capabilities, including default inputs, secret bindings, labels, and
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

## Rules

1. Workflows call actions, not capabilities.
2. Actions should hold project-specific configuration, not distributable code.
3. Runtime overrides should be rare and reviewable.
4. Secret bindings should stay declarative.

## Review Flow

1. Read `docs/architecture/takt-foundations.md`.
2. Inspect `cargo run -- schema action`.
3. Confirm the action is configuration, not implementation.

## Current Command

Use `cargo run -- action init <name> <capability>` to scaffold an action
manifest, then edit the generated YAML.

## Smells

- actions embedding large script bodies
- actions duplicating package logic
- actions bypassing capability permissions
