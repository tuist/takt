Include e2e/spec_helper.sh

default_workflow_manifest_query() {
  local dir="$1"
  local expr="$2"
  run_takt_in "$dir" generate workflow daily-triage --uses github-triage >/dev/null || return $?
  yaml_query "$dir/workflows/daily-triage.yaml" "$expr"
}

custom_workflow_manifest_query() {
  local dir="$1"
  local expr="$2"
  run_takt generate workflow daily-triage --uses github-triage --output "$dir/custom/workflow.yaml" >/dev/null || return $?
  yaml_query "$dir/custom/workflow.yaml" "$expr"
}

Describe 'takt generate workflow'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'creates a workflow scaffold under workflows/ by default'
    When call run_takt_in "$TEST_WORKSPACE" generate workflow daily-triage --uses github-triage
    The status should be success
    The output should include "workflows/daily-triage.yaml"
  End

  It 'writes the expected workflow manifest'
    When call default_workflow_manifest_query "$TEST_WORKSPACE" '.kind'
    The status should be success
    The output should equal "Workflow"
  End

  It 'writes the expected workflow name'
    When call default_workflow_manifest_query "$TEST_WORKSPACE" '.name'
    The status should be success
    The output should equal "daily-triage"
  End

  It 'writes the expected action reference'
    When call default_workflow_manifest_query "$TEST_WORKSPACE" '.steps[0].uses'
    The status should be success
    The output should equal "github-triage"
  End

  It 'writes the expected starter step name'
    When call default_workflow_manifest_query "$TEST_WORKSPACE" '.steps[0].name'
    The status should be success
    The output should equal "step-1"
  End

  It 'supports a custom output path'
    When call custom_workflow_manifest_query "$TEST_WORKSPACE" '.kind'
    The status should be success
    The output should equal "Workflow"
  End

  It 'writes valid YAML to a custom output path'
    When call custom_workflow_manifest_query "$TEST_WORKSPACE" '.name'
    The status should be success
    The output should equal "daily-triage"
  End
End
