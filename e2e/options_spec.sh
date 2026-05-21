Include e2e/spec_helper.sh

takt_help_output() {
  run_takt --help
}

concepts_toon_query_via_env() {
  TAKT_FORMAT=toon run_takt concepts | json_query_stdin "$1"
}

validate_package_via_env_dir() {
  local dir="$1"
  bootstrap_package_for_options "$dir" || return $?
  (
    cd /
    TAKT_DIR="$dir" "$CARGO_BIN" run --quiet --bin takt --manifest-path "$TAKT_ROOT/Cargo.toml" -- --format toon validate package
  ) | json_query_stdin '.passed'
}

init_description_via_env() {
  local dir="$1"
  (
    cd "$dir" &&
      TAKT_INIT_DESCRIPTION="From env" \
      "$CARGO_BIN" run --quiet --bin takt --manifest-path "$TAKT_ROOT/Cargo.toml" -- init @acme/test >/dev/null
  ) || return $?
  yaml_query "$dir/package.yaml" '.package.description'
}

generate_workflow_uses_via_short_flag() {
  local dir="$1"
  run_takt_in "$dir" generate workflow daily-triage -u github-triage >/dev/null || return $?
  yaml_query "$dir/workflows/daily-triage.yaml" '.steps[0].uses'
}

run_action_input_via_env() {
  local dir="$1"
  bootstrap_package_for_options "$dir" || return $?
  (
    cd "$dir" &&
      TAKT_FORMAT=toon \
      TAKT_RUN_ACTION_INPUT="limit=10,dry_run=true" \
      "$CARGO_BIN" run --quiet --bin takt --manifest-path "$TAKT_ROOT/Cargo.toml" -- run action github-triage
  ) | json_query_stdin '.run.inputs.dry_run'
}

mcp_help_output() {
  run_takt mcp --help
}

bootstrap_package_for_options() {
  local dir="$1"
  run_takt_in "$dir" init @acme/test >/dev/null || return $?
  run_takt_in "$dir" generate action github-triage example.run >/dev/null || return $?
}

Describe 'takt option surface'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'shows short global flags in top-level help'
    When call takt_help_output
    The status should be success
    The output should include "-F, --format"
    The output should include "-C, --dir"
    The output should include "TAKT_FORMAT"
    The output should include "TAKT_DIR"
  End

  It 'supports global output format through the environment'
    When call concepts_toon_query_via_env '.concepts[0].name'
    The status should be success
    The output should equal "Package"
  End

  It 'supports package directory through the environment'
    When call validate_package_via_env_dir "$TEST_WORKSPACE"
    The status should be success
    The output should equal "true"
  End

  It 'supports init description through the environment'
    When call init_description_via_env "$TEST_WORKSPACE"
    The status should be success
    The output should equal "From env"
  End

  It 'supports short flags on workflow generation'
    When call generate_workflow_uses_via_short_flag "$TEST_WORKSPACE"
    The status should be success
    The output should equal "github-triage"
  End

  It 'supports run inputs through the environment'
    When call run_action_input_via_env "$TEST_WORKSPACE"
    The status should be success
    The output should equal "true"
  End

  It 'shows short flags and env vars on mcp help'
    When call mcp_help_output
    The status should be success
    The output should include "-t, --transport"
    The output should include "-l, --listen"
    The output should include "-p, --path"
    The output should include "TAKT_MCP_TRANSPORT"
    The output should include "TAKT_MCP_LISTEN"
    The output should include "TAKT_MCP_PATH"
  End
End
