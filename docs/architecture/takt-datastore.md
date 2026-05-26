# Takt Datastore and Artifact Persistence

Takt needs a first-class persistence model.

Today, the prototype only persists planned run records under `.takt/runs/`.
That is useful for inspection, but it is not the same thing as durable
artifact storage, versioned outputs, retention, querying, or cross-run state.

This document proposes a Takt datastore model inspired by Swamp's persistence
shape while keeping Takt's own nouns and boundaries:

- Takt core owns the run and artifact model.
- A datastore is pluggable and can be implemented by a Node module.
- Datastore providers run in a dedicated runtime, separate from action runs.
- Takt should ship built-in providers, with `sqlite` as the default local
  store.

## Goals

- Persist actual run outputs, not only run plans.
- Keep `artifact` as a first-class Takt concept.
- Support local-first operation with optional remote sync later.
- Allow datastore backends to be extended through packages.
- Keep the provider contract narrow enough that Takt retains semantic control.
- Run storage logic in a sandboxed Node process with explicit policy.

## Non-Goals

- Push artifact semantics into arbitrary plugins.
- Make workflows or actions talk to raw storage backends directly.
- Require remote infrastructure for normal local usage.
- Solve distributed execution in the first slice.

## Current Gap

Takt's language already implies persisted outputs:

- `package -> capability -> action -> workflow -> run -> artifact`
- `Artifact`: persisted output from a run

But the current implementation only persists a plan record for `takt run`.

The missing pieces are:

- a repo-level datastore configuration
- a persisted artifact record model
- named artifact declarations in capabilities
- retention and versioning policy
- artifact query and garbage-collection commands
- a backend contract for local and remote storage

## Principles

## Core Owns Semantics

Providers should not define what a run or artifact means.

Takt core should continue to own:

- run identity
- artifact identity
- versioning rules
- retention rules
- record schemas
- workflow and action references
- CLI and MCP query behavior

Providers should only implement storage and coordination primitives.

## Repo Config, Not Package API

Datastore configuration is operational and repository-local.

It should not be part of the publishable package surface in `takt.json`.
Packages publish capabilities. Repositories choose how to persist data
produced by those capabilities.

Recommended direction:

- keep `takt.json` as the package manifest
- add a repo-local config file under `.takt/`

For discussion, this document uses `.takt/config.json`.

## Separate Storage Runtime

Datastore operations have different requirements than capability execution:

- locking across runs
- garbage collection
- retention enforcement
- optional sync
- health checks
- potentially different network policy

That argues for a dedicated storage runtime profile, not reuse of the action
sandbox process.

## Proposed Object Model

Takt's conceptual chain stays the same:

`package -> capability -> action -> workflow -> run -> artifact`

What changes is that `artifact` becomes a real stored object with identity and
policy instead of only a concept in docs.

### Run

A run record should represent an actual execution, not only a plan:

- `id`
- `kind`: `action` or `workflow`
- `status`
- `mode`
- `started_at`
- `finished_at`
- `inputs`
- `repo_root`
- `action` or `workflow` target metadata
- `artifacts`: summary references to persisted artifacts

### Artifact

An artifact record should be addressable on its own:

- `id`
- `run_id`
- `producer_kind`: `capability`, `action`, or `workflow`
- `producer_name`
- `step_name` when produced by a workflow step
- `name`
- `artifact_type`: `resource` or `file`
- `schema_ref` for structured resources when present
- `content_type` for files when present
- `version`
- `tags`
- `created_at`
- `retention`
- `vary`
- `storage_ref`

`storage_ref` is the provider-specific locator for the stored bytes or record.

## Capability Declarations

Today a capability has one `output` schema. That is too narrow for durable
artifact persistence.

Recommended direction:

- keep `output` for the immediate handler result
- add `artifacts` for named persisted outputs

Example:

```json
{
  "capabilities": {
    "scan.repo": {
      "handler": {
        "entrypoint": "handlers/scan.mjs"
      },
      "input": {
        "path": "schemas/scan-input.json"
      },
      "output": {
        "path": "schemas/scan-result.json"
      },
      "artifacts": {
        "summary": {
          "type": "resource",
          "schema": {
            "path": "schemas/scan-summary.json"
          }
        },
        "report": {
          "type": "file",
          "content_type": "application/json"
        }
      }
    }
  }
}
```

This preserves today's single immediate `output` while adding explicit durable
artifacts.

## Workflow Overrides

Workflows should be able to override artifact persistence policy per step.

Example shape:

```json
{
  "name": "daily-triage",
  "steps": [
    {
      "name": "scan",
      "uses": "github-triage",
      "artifacts": {
        "summary": {
          "retention": {
            "lifetime": "30d",
            "keep_latest": 20
          },
          "tags": {
            "kind": "triage"
          },
          "vary": ["inputs.owner", "inputs.repo"]
        }
      }
    }
  ]
}
```

The workflow does not redefine the artifact schema. It only adjusts persistence
policy for that use site.

## Repo Datastore Configuration

Recommended repo-local config:

```json
{
  "datastore": {
    "provider": "sqlite",
    "runtime": "storage",
    "config": {
      "path": ".takt/datastore/catalog.db"
    }
  },
  "runtimes": {
    "storage": {
      "sandbox": "microsandbox",
      "image": "ghcr.io/example/takt-storage@sha256:replace-me",
      "network": {
        "mode": "disabled"
      }
    }
  }
}
```

This keeps storage policy repo-local while still using named runtime profiles.

## Provider Contract

The provider contract should be JavaScript or TypeScript and executed in Node.
Takt core should launch it as a dedicated process inside the configured storage
runtime.

The contract should be narrow and typed.

Illustrative shape:

```ts
export interface DatastoreProvider {
  validateConfig(input: unknown): Promise<void>;
  healthcheck(): Promise<DatastoreHealth>;
  acquireLock(name: string): Promise<DatastoreLock>;
  putRecord(collection: string, key: string, value: unknown): Promise<void>;
  getRecord(collection: string, key: string): Promise<unknown | null>;
  listRecords(query: ListRecordsQuery): Promise<ListRecordsResult>;
  deleteRecord(collection: string, key: string): Promise<void>;
  putBlob(input: PutBlobInput): Promise<StorageRef>;
  getBlob(ref: StorageRef): Promise<ReadableStream | null>;
  deleteBlob(ref: StorageRef): Promise<void>;
  compact?(): Promise<void>;
  sync?(): Promise<SyncResult>;
}
```

Important boundary:

- Takt defines the record collections and their schemas.
- The provider stores and retrieves them.
- The provider does not invent artifact semantics or query language.

That gives us extension flexibility without losing product coherence.

## Why Node Here

Running datastore providers in Node is reasonable for Takt:

- it matches the package execution contract already centered on Node
- it allows bundled and external providers to share one implementation model
- it makes TypeScript provider authoring straightforward
- it keeps custom provider logic out of Rust

The main constraint is process isolation. Providers should run in a dedicated
storage runtime, not inside the same process that executes user capabilities.

## Built-In Providers

Takt should ship at least these providers:

- `sqlite`
- `filesystem`

### `sqlite`

Recommended default.

Use a SQLite catalog for:

- run records
- artifact records
- indexes
- retention metadata
- lock bookkeeping when needed

Blobs can either live in SQLite or in sibling files under `.takt/datastore/`.
The exact blob strategy can stay an implementation decision for the first cut.

### `filesystem`

Useful as the simplest portable backend.

It can store:

- JSON records in stable directory layouts
- file artifacts as plain files
- optional sidecar indexes until query requirements push harder on SQLite

This provider is useful for debugging and low-dependency setups, even if
`sqlite` remains the default.

## Local Layout

Recommended default local layout for the `sqlite` provider:

```text
.takt/
  config.json
  runs/
  datastore/
    catalog.db
    blobs/
```

Notes:

- `.takt/runs/` can remain as a compatibility location during migration
- over time, run records should move into the catalog as well
- `blobs/` can hold file content addressed by artifact record `storage_ref`

## CLI Surface

The datastore proposal implies a new command family:

- `takt artifact get`
- `takt artifact list`
- `takt artifact query`
- `takt artifact versions`
- `takt artifact delete`
- `takt artifact gc`
- `takt datastore status`
- `takt datastore setup`
- `takt datastore compact`
- `takt datastore sync`

`takt run` should also evolve:

- planning mode persists a plan record only
- execution mode persists run records plus artifact records

## MCP Surface

The MCP server should expose the same underlying operations:

- plan a run
- execute a run
- list artifacts
- get artifact metadata
- get artifact content
- run garbage collection
- inspect datastore health

The Rust core should remain the single implementation behind both CLI and MCP.

## Recommended First Slice

The first implementation slice should stay narrow:

1. Add repo-local datastore config under `.takt/config.json`.
2. Add `artifacts` declarations to capabilities.
3. Introduce `ArtifactRecord` and persistent `RunRecord` types in Rust.
4. Implement a built-in `sqlite` provider in Node.
5. Persist run records and artifact records during execution.
6. Add `takt artifact list|get`.

That is enough to validate the model without solving remote sync yet.

## Migration Notes

We do not need to remove the current `.takt/runs/` behavior immediately.

Safer path:

- keep `.takt/runs/` as the prototype run-plan location
- introduce datastore-backed execution records next
- migrate planning records into the datastore once execution exists

## Open Questions

- Should the repo-local config file be `.takt/config.json`, `.takt/repo.json`,
  or something else?
- Should `output` stay as the immediate result schema, or should Takt rename it
  to `result` before execution lands?
- Should `sqlite` store blobs inline, on disk, or support both?
- Does `filesystem` need full query support, or can it be limited to debugging
  and simple local workflows?
- Do we want one provider contract for both local and remote backends, or a
  narrower local contract plus optional sync extensions?

## Recommendation

Adopt the Swamp-like extension shape, but keep Takt's semantics in core.

That means:

- pluggable datastore providers implemented in Node
- a dedicated storage runtime profile
- a repo-level datastore config
- built-in `sqlite` as the default local store
- first-class artifact declarations and records in Takt itself

That gives Takt a real persistence model instead of only persisted plan files,
without turning artifact semantics into backend-specific behavior.
