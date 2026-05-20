---
name: takt-getting-started
description: >
  Onboard an agent to a Takt package repository. Use when the user is new to
  Takt, asks for the core nouns, or needs the package, capability, action, and
  workflow model explained before deeper design work.
---

# Takt Getting Started

Use this skill to keep the local package vocabulary consistent.

## First Steps

1. Read `package.yaml`.
2. Run `takt concepts`.
3. If the current file shapes matter, run `takt schema all`.
4. Explain the nouns in this order:
   `package -> capability -> action -> workflow -> run -> artifact`

## Local Files

- `package.yaml`
- `actions/*.yaml`
- `workflows/*.yaml`

## Routing

- If the user is changing package capabilities or runtimes, use `takt-package`.
- If the user is changing configured uses of capabilities, use `takt-action`.
- If the user is changing orchestration, use `takt-workflow`.

## Non-Negotiables

- Do not collapse `capability` and `action` into one concept.
- Do not let workflows depend on raw images or scripts.
- Treat runtime profiles as reviewed infrastructure, not ad-hoc step settings.
