Include e2e/spec_helper.sh

bootstrap_execution_repo() {
  local dir="$1"
  run_takt_in "$dir" init @acme/test >/dev/null || return $?
  run_takt_in "$dir" generate action github-triage example.run >/dev/null || return $?
  run_takt_in "$dir" generate workflow daily-triage --uses github-triage >/dev/null || return $?
}

package_validation_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json validate package | json_query_stdin "$expr"
}

action_validation_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json validate action github-triage | json_query_stdin "$expr"
}

workflow_validation_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json validate workflow daily-triage | json_query_stdin "$expr"
}

all_validation_query() {
  local dir="$1"
  local expr="$2"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" --format json validate all | json_query_stdin "$expr"
}

invalid_workflow_validation() {
  local dir="$1"
  bootstrap_execution_repo "$dir" || return $?
  run_takt_in "$dir" generate workflow broken-workflow --uses missing-action >/dev/null || return $?
  run_takt_in "$dir" --format json validate workflow broken-workflow
}

Describe 'takt validate'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'validates the package manifest'
    When call package_validation_query "$TEST_WORKSPACE" '.passed'
    The status should be success
    The output should equal "true"
  End

  It 'validates an action manifest'
    When call action_validation_query "$TEST_WORKSPACE" '.subject'
    The status should be success
    The output should equal "github-triage"
  End

  It 'validates a workflow manifest'
    When call workflow_validation_query "$TEST_WORKSPACE" '.passed'
    The status should be success
    The output should equal "true"
  End

  It 'can validate the whole package'
    When call all_validation_query "$TEST_WORKSPACE" '.reports | length'
    The status should be success
    The output should equal "3"
  End

  It 'fails when validation fails'
    When call invalid_workflow_validation "$TEST_WORKSPACE"
    The status should be failure
    The output should include '"passed": false'
    The error should include "workflow validation failed"
  End
End
