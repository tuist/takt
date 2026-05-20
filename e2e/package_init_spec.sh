Include e2e/spec_helper.sh

package_manifest_after_init() {
  local dir="$1"
  run_takt_in "$dir" package init @acme/test --description "Test package" >/dev/null || return $?
  cat_file "$dir/package.yaml"
}

package_init_without_force_fails() {
  local dir="$1"
  printf 'package:\n  name: existing\n' | write_stdin_to "$dir/package.yaml"
  run_takt_in "$dir" package init @acme/test
}

package_manifest_after_force_overwrite() {
  local dir="$1"
  printf 'package:\n  name: existing\n' | write_stdin_to "$dir/package.yaml"
  run_takt_in "$dir" package init @acme/test --force >/dev/null || return $?
  cat_file "$dir/package.yaml"
}

Describe 'takt package init'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'creates a package scaffold at the default path'
    When call run_takt_in "$TEST_WORKSPACE" package init @acme/test --description "Test package"
    The status should be success
    The output should include "Wrote package.yaml"
  End

  It 'writes the expected package manifest'
    When call package_manifest_after_init "$TEST_WORKSPACE"
    The status should be success
    The output should include "api_version: takt.dev/v1alpha1"
    The output should include "kind: Package"
    The output should include "name: '@acme/test'"
    The output should include "sandbox: microsandbox"
    The output should include "example.run:"
  End

  It 'refuses to overwrite an existing manifest without --force'
    When call package_init_without_force_fails "$TEST_WORKSPACE"
    The status should be failure
    The error should include "already exists"
  End

  It 'overwrites an existing manifest with --force'
    When call package_manifest_after_force_overwrite "$TEST_WORKSPACE"
    The status should be success
    The output should include "name: '@acme/test'"
  End
End
