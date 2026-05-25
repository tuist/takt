---
name: takt-action
description: >
  Design or review Takt actions. Use when working on project-local configured
  uses of capabilities, including defaults, secret bindings, labels, and
  ownership metadata.
---

# Takt Action

Actions are project-local configured uses of capabilities.
This skill is a routing guide. Treat `takt schema action --format toon` and the action manifest as the source of truth.

## Critical Rules

- Never write an action manifest from scratch. Always run `takt generate action <name> <capability>` first, then edit the generated JSON.
- Never use an action to smuggle implementation that belongs in a capability handler.
- Always validate after edits with `takt validate action <name-or-path> --format toon`.
- If execution behavior matters, inspect the plan with `takt run action <name> --format toon`.

## Quick Reference

| Task | Command |
| --- | --- |
| Get action schema | `takt schema action --format toon` |
| Generate action | `takt generate action <name> <capability>` |
| Validate action | `takt validate action <name-or-path> --format toon` |
| Plan action run | `takt run action <name> --format toon` |

## Responsibilities

- bind a capability reference
- provide default inputs
- bind secret sources
- attach labels and ownership metadata

## Review Flow

1. If creating a new action, scaffold it first with `takt generate action <name> <capability>`.
2. Read the relevant file under `actions/`.
3. Run `takt schema action --format toon`.
4. Run `takt validate action <name-or-path> --format toon`.
5. Confirm the action is configuration, not implementation.

## Rules

1. Workflows call actions, not capabilities.
2. Actions should hold project-specific configuration, not distributable code.
3. Secret bindings should stay declarative.
4. Prefer existing capabilities over inventing a new one just to fit a single action.

## Current Command

Use `takt generate action <name> <capability>` to scaffold an action manifest,
then edit the generated JSON.

## Smells

- actions embedding large script bodies
- actions duplicating package logic
- actions bypassing capability permissions
