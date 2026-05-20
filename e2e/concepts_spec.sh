Include e2e/spec_helper.sh

Describe 'takt concepts'
  It 'prints the canonical object model'
    When call run_takt concepts
    The status should be success
    The output should include "package -> capability -> action -> workflow -> run -> artifact"
    The output should include "Runtime rule:"
  End

  It 'supports JSON output'
    When call run_takt concepts --json
    The status should be success
    The output should include '"name": "Package"'
    The output should include '"name": "Workflow"'
  End
End
