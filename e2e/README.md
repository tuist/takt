# Takt E2E

This directory holds end-to-end tests for the current Takt CLI surface.

## Scope

Every user-facing command in the current prototype should have an e2e spec:

- `takt concepts`
- `takt schema`
- `takt init` including project bootstrap files
- `takt generate action`
- `takt generate workflow`
- `takt validate`
- `takt run`

As new commands are added, add a matching spec file here.

Generated manifest checks should prefer `yq` queries over raw substring matches
so the suite validates YAML structure, not just text output.

Command output checks should prefer `--format toon` plus `yq` queries for the
same reason.

## Running

```sh
mise run e2e
```

Or directly:

```sh
mise exec -- shellspec
```
