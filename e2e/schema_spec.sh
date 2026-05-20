Include e2e/spec_helper.sh

Describe 'takt schema'
  It 'emits the full schema bundle'
    When call run_takt schema all
    The status should be success
    The output should include '"package"'
    The output should include '"runtime"'
    The output should include '"capability"'
    The output should include '"action"'
    The output should include '"workflow"'
  End

  It 'emits the package schema'
    When call run_takt schema package
    The status should be success
    The output should include '"title": "PackageManifest"'
    The output should include '"capabilities"'
  End

  It 'emits the runtime schema'
    When call run_takt schema runtime
    The status should be success
    The output should include '"title": "RuntimeProfile"'
    The output should include '"sandbox"'
  End

  It 'emits the capability schema'
    When call run_takt schema capability
    The status should be success
    The output should include '"title": "CapabilityDefinition"'
    The output should include '"permissions"'
  End

  It 'emits the action schema'
    When call run_takt schema action
    The status should be success
    The output should include '"title": "ActionDefinition"'
    The output should include '"capability"'
  End

  It 'emits the workflow schema'
    When call run_takt schema workflow
    The status should be success
    The output should include '"title": "WorkflowDefinition"'
    The output should include '"steps"'
  End
End
