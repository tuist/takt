Include e2e/spec_helper.sh

bootstrap_execution_repo() {
  local dir="$1"
  run_takt_in "$dir" init @acme/test >/dev/null || return $?
  run_takt_in "$dir" generate action github-triage example.run >/dev/null || return $?
  run_takt_in "$dir" generate workflow daily-triage --uses github-triage >/dev/null || return $?
}

action_run_plan_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run action github-triage --plan-only --input limit=10 --input dry_run=true |
    json_query_stdin "$expr"
}

action_run_execute_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run action github-triage --input limit=10 |
    json_query_stdin "$expr"
}

workflow_run_plan_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run workflow daily-triage --input limit=10 |
    json_query_stdin "$expr"
}

action_run_datastore_record_status() {
  local dir="$1"
  local run_id
  bootstrap_execution_repo "$dir" || return $?
  run_id="$(run_takt_in "$dir" --format json run action github-triage --plan-only | json_query_stdin '.run.id')" || return $?
  if [ -f "$dir/.takt/datastore/runs/${run_id}.json" ]; then
    printf 'present\n'
  else
    printf 'missing\n'
    return 1
  fi
}

workflow_run_without_persist_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run workflow daily-triage --no-persist |
    json_query_stdin "$expr"
}

workflow_run_without_persist_datastore_count() {
  local dir="$1"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run workflow daily-triage --no-persist >/dev/null || return $?
  find "$dir/.takt/datastore/runs" -type f -name '*.json' 2>/dev/null | wc -l | tr -d ' '
}

workflow_execute_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run workflow daily-triage |
    json_query_stdin "$expr"
}

run_list_query() {
  local dir="$1"
  local expr="$2"
  shift 2
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run action github-triage --plan-only >/dev/null || return $?
  run_takt_in "$dir" --format json run workflow daily-triage --plan-only >/dev/null || return $?
  run_takt_in "$dir" --format json run list "$@" | json_query_stdin "$expr"
}

run_get_query() {
  local dir="$1"
  local expr="$2"
  local run_id
  bootstrap_execution_repo "$dir" || return $?
  run_id="$(run_takt_in "$dir" --format json run action github-triage --plan-only | json_query_stdin '.run.id')" || return $?
  run_takt_in "$dir" --format json run get "$run_id" | json_query_stdin "$expr"
}

run_get_missing_status() {
  local dir="$1"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" run get bogus
}

Describe 'takt run'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'plans an action run with --plan-only'
    When call action_run_plan_query "$TEST_WORKSPACE" '.run.status'
    The status should be success
    The output should equal "planned"
  End

  It 'executes an action run by default and emits a succeeded record'
    When call action_run_execute_query "$TEST_WORKSPACE" '.run.status'
    The status should be success
    The output should equal "succeeded"
  End

  It 'captures the handler output on the executed run'
    When call action_run_execute_query "$TEST_WORKSPACE" '.run.output.greeting'
    The status should be success
    The output should equal "hello from example.run"
  End

  It 'reports the emitted artifact id on the executed run'
    When call action_run_execute_query "$TEST_WORKSPACE" '.run.artifact_ids | length'
    The status should be success
    The output should equal "1"
  End

  It 'executes a workflow by default and emits a succeeded record'
    When call workflow_execute_query "$TEST_WORKSPACE" '.run.status'
    The status should be success
    The output should equal "succeeded"
  End

  It 'reports child step run ids on the executed workflow'
    When call workflow_execute_query "$TEST_WORKSPACE" '.run.child_run_ids | length'
    The status should be success
    The output should equal "1"
  End

  It 'aggregates step outputs on the executed workflow'
    When call workflow_execute_query "$TEST_WORKSPACE" '.run.output["step-1"].greeting'
    The status should be success
    The output should equal "hello from example.run"
  End

  It 'parses action run inputs as structured JSON values'
    When call action_run_plan_query "$TEST_WORKSPACE" '.run.inputs.limit'
    The status should be success
    The output should equal "10"
  End

  It 'resolves local action capabilities when planning a run'
    When call action_run_plan_query "$TEST_WORKSPACE" '.run.action.resolution.mode'
    The status should be success
    The output should equal "local"
  End

  It 'persists planned action runs in the datastore catalog'
    When call action_run_datastore_record_status "$TEST_WORKSPACE"
    The status should be success
    The output should equal "present"
  End

  It 'reports persisted=true when planning persisted runs'
    When call action_run_plan_query "$TEST_WORKSPACE" '.run.persisted'
    The status should be success
    The output should equal "true"
  End

  It 'plans a workflow run'
    When call workflow_run_plan_query "$TEST_WORKSPACE" '.run.workflow.steps[0].action'
    The status should be success
    The output should equal "github-triage"
  End

  It 'reports persisted=false when --no-persist is passed'
    When call workflow_run_without_persist_query "$TEST_WORKSPACE" '.run.persisted'
    The status should be success
    The output should equal "false"
  End

  It 'does not write a datastore record when --no-persist is passed'
    When call workflow_run_without_persist_datastore_count "$TEST_WORKSPACE"
    The status should be success
    The output should equal "0"
  End

  It 'lists persisted runs with the canonical envelope'
    When call run_list_query "$TEST_WORKSPACE" '.command'
    The status should be success
    The output should equal "run list"
  End

  It 'returns both planned runs when no filter is applied'
    When call run_list_query "$TEST_WORKSPACE" '.total'
    The status should be success
    The output should equal "2"
  End

  It 'filters runs by kind'
    When call run_list_query "$TEST_WORKSPACE" '.total' --kind action
    The status should be success
    The output should equal "1"
  End

  It 'marks the envelope as limited when --limit truncates results'
    When call run_list_query "$TEST_WORKSPACE" '.limited' --limit 1
    The status should be success
    The output should equal "true"
  End

  It 'gets a single persisted run by id'
    When call run_get_query "$TEST_WORKSPACE" '.run.kind'
    The status should be success
    The output should equal "action"
  End

  It 'reports a missing run as an error'
    When call run_get_missing_status "$TEST_WORKSPACE"
    The status should be failure
    The error should include "was not found in the datastore"
  End
End
