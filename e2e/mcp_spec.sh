Include e2e/spec_helper.sh

mcp_initialize_query() {
  local dir="$1"
  local expr="$2"
  local headers_file="$dir/initialize.headers"
  local body_file="$dir/initialize.body"

  start_takt_mcp_http "$dir" || return $?
  local url
  url="$(mcp_http_url "$dir")"

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"shellspec","version":"1.0"}}}' \
    "$headers_file" \
    "$body_file" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  response_json_from_file "$body_file" | json_query_stdin "$expr"
  local status=$?
  stop_takt_mcp_http "$dir"
  return "$status"
}

mcp_initialize_session_id() {
  local dir="$1"
  local headers_file="$dir/initialize.headers"
  local body_file="$dir/initialize.body"

  start_takt_mcp_http "$dir" || return $?
  local url
  url="$(mcp_http_url "$dir")"

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"shellspec","version":"1.0"}}}' \
    "$headers_file" \
    "$body_file" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  http_header_value "$headers_file" "mcp-session-id"
  local status=$?
  stop_takt_mcp_http "$dir"
  return "$status"
}

mcp_tools_list_query() {
  local dir="$1"
  local expr="$2"
  local initialize_headers="$dir/initialize.headers"
  local initialize_body="$dir/initialize.body"
  local initialized_headers="$dir/initialized.headers"
  local initialized_body="$dir/initialized.body"
  local list_headers="$dir/list.headers"
  local list_body="$dir/list.body"

  start_takt_mcp_http "$dir" || return $?
  local url
  url="$(mcp_http_url "$dir")"

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"shellspec","version":"1.0"}}}' \
    "$initialize_headers" \
    "$initialize_body" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  local session_id
  session_id="$(http_header_value "$initialize_headers" "mcp-session-id")"

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    "$initialized_headers" \
    "$initialized_body" \
    -H "mcp-session-id: $session_id" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
    "$list_headers" \
    "$list_body" \
    -H "mcp-session-id: $session_id" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  response_json_from_file "$list_body" | json_query_stdin "$expr"
  local status=$?
  stop_takt_mcp_http "$dir"
  return "$status"
}

mcp_package_init_without_agents() {
  local dir="$1"
  local initialize_headers="$dir/initialize.headers"
  local initialize_body="$dir/initialize.body"
  local initialized_headers="$dir/initialized.headers"
  local initialized_body="$dir/initialized.body"
  local call_headers="$dir/call.headers"
  local call_body="$dir/call.body"
  local output="$dir/custom/takt.json"

  start_takt_mcp_http "$dir" || return $?
  local url
  url="$(mcp_http_url "$dir")"

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"shellspec","version":"1.0"}}}' \
    "$initialize_headers" \
    "$initialize_body" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  local session_id
  session_id="$(http_header_value "$initialize_headers" "mcp-session-id")"

  mcp_http_post "$url" \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    "$initialized_headers" \
    "$initialized_body" \
    -H "mcp-session-id: $session_id" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  mcp_http_post "$url" \
    "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"package_init\",\"arguments\":{\"name\":\"@acme/test\",\"output\":\"$output\",\"coding_agent\":\"none\"}}}" \
    "$call_headers" \
    "$call_body" \
    -H "mcp-session-id: $session_id" >/dev/null || {
    stop_takt_mcp_http "$dir"
    return 1
  }

  if [ ! -f "$output" ]; then
    stop_takt_mcp_http "$dir"
    return 1
  fi

  if [ -e "$dir/custom/AGENTS.md" ] || [ -d "$dir/custom/.agents" ]; then
    stop_takt_mcp_http "$dir"
    return 1
  fi

  response_json_from_file "$call_body" | json_query_stdin '.result.structuredContent.coding_agent'
  local status=$?
  stop_takt_mcp_http "$dir"
  return "$status"
}

Describe 'takt mcp HTTP transport'
  BeforeEach 'setup_workspace'
  AfterEach 'cleanup_workspace'

  It 'returns initialize metadata over HTTP'
    When call mcp_initialize_query "$TEST_WORKSPACE" '.result.serverInfo.name'
    The status should be success
    The output should equal "takt"
  End

  It 'returns a session id header on initialize'
    When call mcp_initialize_session_id "$TEST_WORKSPACE"
    The status should be success
    The output should not equal ""
  End

  It 'lists the package_init tool over HTTP'
    When call mcp_tools_list_query "$TEST_WORKSPACE" '.result.tools[] | select(.name == "package_init") | .description'
    The status should be success
    The output should include "Initialize a Takt package"
  End

  It 'lists the run_list datastore tool over HTTP'
    When call mcp_tools_list_query "$TEST_WORKSPACE" '.result.tools[] | select(.name == "run_list") | .description'
    The status should be success
    The output should include "persisted runs"
  End

  It 'lists the run_get datastore tool over HTTP'
    When call mcp_tools_list_query "$TEST_WORKSPACE" '.result.tools[] | select(.name == "run_get") | .description'
    The status should be success
    The output should include "single persisted run"
  End

  It 'lists the artifact_list datastore tool over HTTP'
    When call mcp_tools_list_query "$TEST_WORKSPACE" '.result.tools[] | select(.name == "artifact_list") | .description'
    The status should be success
    The output should include "artifacts persisted"
  End

  It 'lists the artifact_get datastore tool over HTTP'
    When call mcp_tools_list_query "$TEST_WORKSPACE" '.result.tools[] | select(.name == "artifact_get") | .description'
    The status should be success
    The output should include "single artifact record"
  End

  It 'can call package_init over HTTP without bootstrapping coding-agent files'
    When call mcp_package_init_without_agents "$TEST_WORKSPACE"
    The status should be success
    The output should equal "none"
  End
End
