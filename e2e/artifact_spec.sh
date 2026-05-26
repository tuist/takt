Include e2e/spec_helper.sh

bootstrap_artifact_repo() {
  local dir="$1"
  run_takt_in "$dir" init @acme/test >/dev/null || return $?
  run_takt_in "$dir" generate action github-triage example.run >/dev/null || return $?
}

datastore_run_file_count() {
  local dir="$1"
  bootstrap_artifact_repo "$dir" || return $?
  run_takt_in "$dir" --format json run action github-triage --plan-only >/dev/null || return $?
  find "$dir/.takt/datastore/runs" -type f -name '*.json' | wc -l | tr -d ' '
}

artifact_list_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_artifact_repo "$dir" || return $?
  # Plan-only avoids invoking the handler so the catalog stays empty.
  run_takt_in "$dir" --format json run action github-triage --plan-only >/dev/null || return $?
  run_takt_in "$dir" --format json artifact list | json_query_stdin "$expr"
}

artifact_list_with_filter_query() {
  local dir="$1"
  local expr="$2"
  shift 2
  bootstrap_artifact_repo "$dir" || return $?
  run_takt_in "$dir" --format json run action github-triage --plan-only >/dev/null || return $?
  run_takt_in "$dir" --format json artifact list "$@" | json_query_stdin "$expr"
}

artifact_list_after_execute_query() {
  local dir="$1"
  local expr="$2"
  shift 2
  bootstrap_artifact_repo "$dir" || return $?
  run_takt_in "$dir" --format json run action github-triage >/dev/null || return $?
  run_takt_in "$dir" --format json artifact list "$@" | json_query_stdin "$expr"
}

artifact_list_invalid_predicate() {
  local dir="$1"
  bootstrap_artifact_repo "$dir" || return $?
  run_takt_in "$dir" artifact list --where "bogus"
}

artifact_get_missing_status() {
  local dir="$1"
  bootstrap_artifact_repo "$dir" || return $?
  run_takt_in "$dir" artifact get bogus
}

Describe 'takt artifact'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'persists a run record into the datastore on takt run'
    When call datastore_run_file_count "$TEST_WORKSPACE"
    The status should be success
    The output should equal "1"
  End

  It 'lists an empty artifact catalog when no handler has executed'
    When call artifact_list_query "$TEST_WORKSPACE" '.total'
    The status should be success
    The output should equal "0"
  End

  It 'returns the canonical list envelope shape'
    When call artifact_list_query "$TEST_WORKSPACE" '.command'
    The status should be success
    The output should equal "artifact list"
  End

  It 'reports limited=false when the catalog fits the limit'
    When call artifact_list_query "$TEST_WORKSPACE" '.limited'
    The status should be success
    The output should equal "false"
  End

  It 'accepts structured filters'
    When call artifact_list_with_filter_query "$TEST_WORKSPACE" '.total' --capability example.run --tag env=prod --since 1h --limit 5
    The status should be success
    The output should equal "0"
  End

  It 'rejects malformed predicates'
    When call artifact_list_invalid_predicate "$TEST_WORKSPACE"
    The status should be failure
    The error should include "must look like path=value"
  End

  It 'reports a missing artifact as an error'
    When call artifact_get_missing_status "$TEST_WORKSPACE"
    The status should be failure
    The error should include "was not found in the datastore"
  End

  It 'persists an artifact when the starter handler executes'
    When call artifact_list_after_execute_query "$TEST_WORKSPACE" '.total'
    The status should be success
    The output should equal "1"
  End

  It 'tags executed-handler artifacts with the handler-provided tag'
    When call artifact_list_after_execute_query "$TEST_WORKSPACE" '.results[0].tags.kind' --capability example.run
    The status should be success
    The output should equal "example"
  End

  It 'filters executed artifacts with a where predicate'
    When call artifact_list_after_execute_query "$TEST_WORKSPACE" '.total' --where producer_kind=capability --where name=summary
    The status should be success
    The output should equal "1"
  End

  It 'returns zero artifacts when the predicate excludes everything'
    When call artifact_list_after_execute_query "$TEST_WORKSPACE" '.total' --where producer_kind=workflow
    The status should be success
    The output should equal "0"
  End
End
