Include e2e/spec_helper.sh

schema_default_query() {
  local expr="$1"
  run_takt schema all | json_query_stdin "$expr"
}

schema_json_query() {
  local expr="$1"
  run_takt --format json schema all | json_query_stdin "$expr"
}

Describe 'takt schema'
  It 'emits the full schema bundle as JSON by default'
    When call schema_default_query '.package.title'
    The status should be success
    The output should equal "PackageManifest"
  End

  It 'supports JSON output through the global format flag'
    When call schema_json_query '.runtime.title'
    The status should be success
    The output should equal "RuntimeProfile"
  End

  It 'emits the package schema'
    When call run_takt schema package
    The status should be success
    The output should include "\"title\": \"PackageManifest\""
    The output should include "\"capabilities\""
  End

  It 'emits the runtime schema'
    When call run_takt schema runtime
    The status should be success
    The output should include "\"title\": \"RuntimeProfile\""
    The output should include "\"sandbox\""
  End

  It 'emits the capability schema'
    When call run_takt schema capability
    The status should be success
    The output should include "\"title\": \"CapabilityDefinition\""
    The output should include "\"permissions\""
  End

  It 'emits the action schema'
    When call run_takt schema action
    The status should be success
    The output should include "\"title\": \"ActionDefinition\""
    The output should include "\"capability\""
  End

  It 'emits the workflow schema'
    When call run_takt schema workflow
    The status should be success
    The output should include "\"title\": \"WorkflowDefinition\""
    The output should include "\"steps\""
  End
End
