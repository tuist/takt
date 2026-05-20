---
name: takt-getting-started
description: >
  Draft onboarding skill for Takt. Use when a user is new to Takt, asks for
  the core nouns, or needs the package, capability, action, and workflow model
  explained before deeper design work.
---

# Takt Getting Started

This is a draft skill for the Takt prototype.

## Goals

Keep the product vocabulary consistent while the CLI and file formats are still
under construction.

## First Steps

1. Read `docs/architecture/takt-foundations.md`.
2. If the current prototype shape matters, run `cargo run -- schema all`.
3. If the user is asking about nouns, explain them in this order:
   `package -> capability -> action -> workflow -> run -> artifact`

## Current Commands

- `cargo run -- concepts`
- `cargo run -- schema all`
- `cargo run -- package init <name>`
- `cargo run -- action init <name> <capability>`
- `cargo run -- workflow init <name> --uses <action>`

## Routing

- If the user is designing registry objects, use `takt-package`.
- If the user is designing project-local configured execution, use
  `takt-action`.
- If the user is designing orchestration, use `takt-workflow`.

## Non-Negotiables

- Do not collapse `capability` and `action` into one concept.
- Do not let workflows depend on raw images or scripts.
- Treat runtime profiles as reviewed infrastructure, not ad-hoc step settings.
