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
  run_takt_in "$dir" --format json run action github-triage --input limit=10 --input dry_run=true |
    json_query_stdin "$expr"
}

workflow_run_plan_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json run workflow daily-triage --input limit=10 |
    json_query_stdin "$expr"
}

action_run_state_file_status() {
  local dir="$1"
  local state_path
  bootstrap_execution_repo "$dir" || return $?
  state_path="$(run_takt_in "$dir" --format json run action github-triage | json_query_stdin '.run.state_path')" || return $?
  if [ -f "$state_path" ]; then
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

Describe 'takt run'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'plans an action run'
    When call action_run_plan_query "$TEST_WORKSPACE" '.run.status'
    The status should be success
    The output should equal "planned"
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

  It 'persists planned action runs under .takt/runs'
    When call action_run_state_file_status "$TEST_WORKSPACE"
    The status should be success
    The output should equal "present"
  End

  It 'plans a workflow run'
    When call workflow_run_plan_query "$TEST_WORKSPACE" '.run.workflow.steps[0].action'
    The status should be success
    The output should equal "github-triage"
  End

  It 'can skip persistence for workflow runs'
    When call workflow_run_without_persist_query "$TEST_WORKSPACE" '.run.state_path'
    The status should be success
    The output should equal "null"
  End
End
