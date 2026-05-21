Include e2e/spec_helper.sh

package_manifest_query_after_init() {
  local dir="$1"
  local expr="$2"
  run_takt_in "$dir" init @acme/test --description "Test package" >/dev/null || return $?
  yaml_query "$dir/package.yaml" "$expr"
}

init_json_query() {
  local dir="$1"
  local expr="$2"
  run_takt_in "$dir" --format json init @acme/test --description "Test package" |
    json_query_stdin "$expr"
}

agents_guide_after_init() {
  local dir="$1"
  run_takt_in "$dir" init @acme/test >/dev/null || return $?
  cat_file "$dir/AGENTS.md"
}

action_skill_after_init() {
  local dir="$1"
  run_takt_in "$dir" init @acme/test >/dev/null || return $?
  cat_file "$dir/.agents/skills/takt-action/SKILL.md"
}

custom_root_agents_after_init() {
  local dir="$1"
  run_takt init @acme/test --output "$dir/bootstrap/package.yaml" >/dev/null || return $?
  cat_file "$dir/bootstrap/AGENTS.md"
}

package_init_without_force_fails() {
  local dir="$1"
  printf '# existing\n' | write_stdin_to "$dir/AGENTS.md"
  run_takt_in "$dir" init @acme/test
}

agents_guide_after_force_overwrite() {
  local dir="$1"
  printf '# existing\n' | write_stdin_to "$dir/AGENTS.md"
  run_takt_in "$dir" init @acme/test --force >/dev/null || return $?
  cat_file "$dir/AGENTS.md"
}

Describe 'takt init'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'creates a package scaffold at the default path'
    When call run_takt_in "$TEST_WORKSPACE" init @acme/test --description "Test package"
    The status should be success
    The output should include "Wrote package.yaml"
    The output should include "Wrote AGENTS.md"
    The output should include ".agents/skills/takt-getting-started/SKILL.md"
  End

  It 'writes the expected package manifest'
    When call package_manifest_query_after_init "$TEST_WORKSPACE" '.kind'
    The status should be success
    The output should equal "Package"
  End

  It 'writes the expected package name'
    When call package_manifest_query_after_init "$TEST_WORKSPACE" '.package.name'
    The status should be success
    The output should equal "@acme/test"
  End

  It 'writes the expected package description'
    When call package_manifest_query_after_init "$TEST_WORKSPACE" '.package.description'
    The status should be success
    The output should equal "Test package"
  End

  It 'emits structured JSON when requested'
    When call init_json_query "$TEST_WORKSPACE" '.package.package.name'
    The status should be success
    The output should equal "@acme/test"
  End

  It 'reports written files in JSON output'
    When call init_json_query "$TEST_WORKSPACE" '.files[0].path'
    The status should be success
    The output should equal "package.yaml"
  End

  It 'writes the expected runtime sandbox'
    When call package_manifest_query_after_init "$TEST_WORKSPACE" '.runtimes.default.sandbox'
    The status should be success
    The output should equal "microsandbox"
  End

  It 'writes the expected capability runtime binding'
    When call package_manifest_query_after_init "$TEST_WORKSPACE" '.capabilities."example.run".runtime'
    The status should be success
    The output should equal "default"
  End

  It 'bootstraps a project-local AGENTS guide'
    When call agents_guide_after_init "$TEST_WORKSPACE"
    The status should be success
    The output should include "This repository is a Takt package named"
    The output should include "@acme/test"
    The output should include "takt concepts"
    The output should include ".agents/skills/takt-action/SKILL.md"
  End

  It 'bootstraps project-local skills for agents'
    When call action_skill_after_init "$TEST_WORKSPACE"
    The status should be success
    The output should include "Actions are project-local configured uses of capabilities."
    The output should include "takt schema action"
    The output should include "Workflows call actions, not capabilities."
  End

  It 'writes bootstrap files relative to a custom manifest path'
    When call custom_root_agents_after_init "$TEST_WORKSPACE"
    The status should be success
    The output should include "This repository is a Takt package named"
    The output should include "@acme/test"
  End

  It 'refuses to overwrite an existing manifest without --force'
    When call package_init_without_force_fails "$TEST_WORKSPACE"
    The status should be failure
    The error should include "already exists"
  End

  It 'overwrites existing bootstrap files with --force'
    When call agents_guide_after_force_overwrite "$TEST_WORKSPACE"
    The status should be success
    The output should include "This repository is a Takt package named"
    The output should include "@acme/test"
  End
End
