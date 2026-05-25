---
name: takt-getting-started
description: >
  Interactive getting-started walkthrough for new Takt users. Guides through
  inspecting the package, understanding the core nouns, scaffolding a first
  action or workflow, and validating the result. Triggers on "getting
  started", "new to takt", "first time", "walkthrough", "onboarding",
  "quickstart", "how do I start", "what do I do first", "learn takt", or
  whenever the user needs the package, capability, action, and workflow model
  explained before deeper work.
---

# Takt Getting Started

A state machine. Each state gates the next. Do not advance until the current
state's Verify passes. If Verify fails, fix the issue and re-verify. If unsure a
subcommand or flag exists, run `takt --help` or `takt <command> --help` rather
than guessing.

```
start -> package_inspected -> concepts_understood -> first_artifact_scaffolded
      -> graduated
```

Prefer CLI TOON output over prose in this file when the command can answer the
question directly.

## Before Starting

1. Read `takt.json`.
2. Run `takt validate package --format toon`.
3. If the package already has actions or workflows, run `takt validate all --format toon`.
4. If the user already speaks in Takt terms, skip ahead to the matching skill.

## Local Files

- `takt.json`
- `actions/*.json`
- `workflows/*.json`

## State 1: package_inspected

Action:

1. Read `takt.json`.
2. Identify the package name, pinned Node version, and existing capabilities.

Verify:

- You can name the package.
- You can point to the package Node version and the capability definitions that already exist.

## State 2: concepts_understood

Action:

1. Run `takt concepts --format toon`.
2. Explain the nouns in this order:
   `package -> capability -> action -> workflow -> run -> artifact`
3. Be explicit that capabilities are reusable interfaces and actions are the
   package-local configured uses of those capabilities.

Verify:

- The user can move forward with the Takt vocabulary for the current task.

## State 3: first_artifact_scaffolded

Action:

- If the user needs a configured capability use, run `takt generate action <name> <capability>`.
- If the user needs orchestration, run `takt generate workflow <name> --uses <action>`.
- After scaffolding, validate with `takt validate action <name-or-path> --format toon` or `takt validate workflow <name-or-path> --format toon`.

Verify:

- The created action or workflow validates successfully.

## State 4: graduated

Action:

- Summarize what exists in the package now.
- Route the next step to `takt-package`, `takt-action`, or `takt-workflow`.

## Routing

- If the user is changing package capabilities or the package Node version, use `takt-package`.
- If the user is changing configured uses of capabilities, use `takt-action`.
- If the user is changing orchestration, use `takt-workflow`.

## Non-Negotiables

- Do not collapse `capability` and `action` into one concept.
- Do not let workflows depend on raw images or scripts.
- Treat the package Node version as the execution contract, not a step-level setting.
