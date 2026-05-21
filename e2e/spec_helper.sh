TAKT_ROOT="${SHELLSPEC_PROJECT_ROOT:-$(pwd)}"
CARGO_BIN="$(mise which cargo)"
YQ_BIN="$(mise which yq)"
CURL_BIN="$(command -v curl)"

run_takt() {
  "$CARGO_BIN" run --quiet --bin takt --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
}

run_takt_in() {
  local dir="$1"
  shift
  (
    cd "$dir" &&
      "$CARGO_BIN" run --quiet --bin takt --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
  )
}

run_takt_mcp() {
  "$CARGO_BIN" run --quiet --bin takt-mcp --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
}

build_takt_mcp() {
  "$CARGO_BIN" build --quiet --bin takt-mcp --manifest-path "$TAKT_ROOT/Cargo.toml"
}

new_workspace() {
  mktemp -d "${TMPDIR:-/tmp}/takt-e2e.XXXXXX"
}

cat_file() {
  cat "$1"
}

yaml_query() {
  local file="$1"
  local expr="$2"
  "$YQ_BIN" -r "$expr" "$file"
}

json_query_stdin() {
  local expr="$1"
  "$YQ_BIN" -p=json -r "$expr" -
}

yaml_query_stdin() {
  local expr="$1"
  "$YQ_BIN" -r "$expr" -
}

write_stdin_to() {
  mkdir -p "$(dirname "$1")"
  cat >"$1"
}

setup_workspace() {
  TEST_WORKSPACE="$(new_workspace)"
}

cleanup_workspace() {
  stop_takt_mcp_http "$TEST_WORKSPACE"
  rm -rf "$TEST_WORKSPACE"
}

wait_for_mcp_http_url() {
  local log_file="$1"
  local attempt=0

  while [ "$attempt" -lt 50 ]; do
    if [ -f "$log_file" ]; then
      local url
      url="$(sed -n 's/^Listening on //p' "$log_file" | tail -n 1)"
      if [ -n "$url" ]; then
        printf '%s' "$url"
        return 0
      fi
    fi

    sleep 0.1
    attempt=$((attempt + 1))
  done

  return 1
}

http_status_from_headers() {
  awk 'NR == 1 { print $2 }' "$1"
}

http_header_value() {
  local header_file="$1"
  local header_name="$2"

  awk -v wanted="$(printf '%s' "$header_name" | tr '[:upper:]' '[:lower:]')" '
    {
      lower = tolower($0)
      if (index(lower, wanted ":") == 1) {
        sub(/^[^:]+:[[:space:]]*/, "", $0)
        sub(/\r$/, "", $0)
        print
        exit
      }
    }
  ' "$header_file"
}

response_json_from_file() {
  local body_file="$1"

  if grep -q '^data: ' "$body_file"; then
    sed -n 's/^data: //p' "$body_file" | tail -n 1
  else
    cat "$body_file"
  fi
}

start_takt_mcp_http() {
  local dir="$1"
  local log_file="$dir/takt-mcp.log"
  local out_file="$dir/takt-mcp.out"
  local bin_path="$TAKT_ROOT/target/debug/takt-mcp"

  build_takt_mcp || return $?
  "$bin_path" --transport http --listen 127.0.0.1:0 --path /mcp >"$out_file" 2>"$log_file" &
  local pid=$!
  local url
  url="$(wait_for_mcp_http_url "$log_file")" || {
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    return 1
  }

  printf '%s\n%s\n' "$pid" "$url" >"$dir/takt-mcp.meta"
}

stop_takt_mcp_http() {
  local dir="$1"
  local pid

  pid="$(sed -n '1p' "$dir/takt-mcp.meta" 2>/dev/null)"
  if [ -n "$pid" ]; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}

mcp_http_url() {
  sed -n '2p' "$1/takt-mcp.meta"
}

mcp_http_post() {
  local url="$1"
  local payload="$2"
  local headers_file="$3"
  local body_file="$4"
  shift 4

  "$CURL_BIN" -sS \
    -D "$headers_file" \
    -o "$body_file" \
    -X POST \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    "$@" \
    --data "$payload" \
    "$url"
}
