Include e2e/spec_helper.sh

default_action_manifest_query() {
  local dir="$1"
  local expr="$2"
  run_takt_in "$dir" generate action github-triage @tuist/github#issues.list >/dev/null || return $?
  yaml_query "$dir/actions/github-triage.yaml" "$expr"
}

custom_action_manifest_query() {
  local dir="$1"
  local expr="$2"
  run_takt generate action github-triage @tuist/github#issues.list --output "$dir/custom/action.yaml" >/dev/null || return $?
  yaml_query "$dir/custom/action.yaml" "$expr"
}

action_json_query() {
  local dir="$1"
  local expr="$2"
  run_takt_in "$dir" --format json generate action github-triage @tuist/github#issues.list |
    json_query_stdin "$expr"
}

Describe 'takt generate action'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'creates an action scaffold under actions/ by default'
    When call run_takt_in "$TEST_WORKSPACE" generate action github-triage @tuist/github#issues.list
    The status should be success
    The output should include "actions/github-triage.yaml"
  End

  It 'writes the expected action manifest'
    When call default_action_manifest_query "$TEST_WORKSPACE" '.kind'
    The status should be success
    The output should equal "Action"
  End

  It 'writes the expected action name'
    When call default_action_manifest_query "$TEST_WORKSPACE" '.name'
    The status should be success
    The output should equal "github-triage"
  End

  It 'writes the expected capability reference'
    When call default_action_manifest_query "$TEST_WORKSPACE" '.capability'
    The status should be success
    The output should equal "@tuist/github#issues.list"
  End

  It 'emits structured JSON when requested'
    When call action_json_query "$TEST_WORKSPACE" '.action.capability'
    The status should be success
    The output should equal "@tuist/github#issues.list"
  End

  It 'reports the generated file path in JSON output'
    When call action_json_query "$TEST_WORKSPACE" '.files[0].path'
    The status should be success
    The output should equal "actions/github-triage.yaml"
  End

  It 'supports a custom output path'
    When call custom_action_manifest_query "$TEST_WORKSPACE" '.kind'
    The status should be success
    The output should equal "Action"
  End

  It 'writes valid YAML to a custom output path'
    When call custom_action_manifest_query "$TEST_WORKSPACE" '.name'
    The status should be success
    The output should equal "github-triage"
  End
End
