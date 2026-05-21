Include e2e/spec_helper.sh

concepts_toon_query() {
  local expr="$1"
  run_takt --format toon concepts | json_query_stdin "$expr"
}

concepts_json_query() {
  local expr="$1"
  run_takt --format json concepts | json_query_stdin "$expr"
}

Describe 'takt concepts'
  It 'prints the canonical object model'
    When call run_takt concepts
    The status should be success
    The output should include "package -> capability -> action -> workflow -> run -> artifact"
    The output should include "Runtime rule:"
  End

  It 'supports JSON output through the global format flag'
    When call concepts_json_query '.concepts[0].name'
    The status should be success
    The output should equal "Package"
  End

  It 'includes the full concept chain in JSON output'
    When call concepts_json_query '.chain'
    The status should be success
    The output should equal "package -> capability -> action -> workflow -> run -> artifact"
  End

  It 'supports TOON output through the global format flag'
    When call concepts_toon_query '.concepts[0].name'
    The status should be success
    The output should equal "Package"
  End
End
