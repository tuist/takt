Include e2e/spec_helper.sh

default_action_manifest() {
  local dir="$1"
  run_takt_in "$dir" generate action github-triage @tuist/github#issues.list >/dev/null || return $?
  cat_file "$dir/actions/github-triage.yaml"
}

custom_action_manifest() {
  local dir="$1"
  run_takt generate action github-triage @tuist/github#issues.list --output "$dir/custom/action.yaml" >/dev/null || return $?
  cat_file "$dir/custom/action.yaml"
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
    When call default_action_manifest "$TEST_WORKSPACE"
    The status should be success
    The output should include "kind: Action"
    The output should include "name: github-triage"
    The output should include "capability: '@tuist/github#issues.list'"
  End

  It 'supports a custom output path'
    When call custom_action_manifest "$TEST_WORKSPACE"
    The status should be success
    The output should include "kind: Action"
    The output should include "name: github-triage"
  End
End
