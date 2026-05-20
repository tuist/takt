Include e2e/spec_helper.sh

default_workflow_manifest() {
  local dir="$1"
  run_takt_in "$dir" workflow init daily-triage --uses github-triage >/dev/null || return $?
  cat_file "$dir/workflows/daily-triage.yaml"
}

custom_workflow_manifest() {
  local dir="$1"
  run_takt workflow init daily-triage --uses github-triage --output "$dir/custom/workflow.yaml" >/dev/null || return $?
  cat_file "$dir/custom/workflow.yaml"
}

Describe 'takt workflow init'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'creates a workflow scaffold under workflows/ by default'
    When call run_takt_in "$TEST_WORKSPACE" workflow init daily-triage --uses github-triage
    The status should be success
    The output should include "workflows/daily-triage.yaml"
  End

  It 'writes the expected workflow manifest'
    When call default_workflow_manifest "$TEST_WORKSPACE"
    The status should be success
    The output should include "kind: Workflow"
    The output should include "name: daily-triage"
    The output should include "uses: github-triage"
    The output should include "name: step-1"
  End

  It 'supports a custom output path'
    When call custom_workflow_manifest "$TEST_WORKSPACE"
    The status should be success
    The output should include "kind: Workflow"
    The output should include "name: daily-triage"
  End
End
