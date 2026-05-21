TAKT_ROOT="${SHELLSPEC_PROJECT_ROOT:-$(pwd)}"
CARGO_BIN="$(mise which cargo)"
YQ_BIN="$(mise which yq)"

run_takt() {
  "$CARGO_BIN" run --quiet --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
}

run_takt_in() {
  local dir="$1"
  shift
  (
    cd "$dir" &&
      "$CARGO_BIN" run --quiet --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
  )
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

write_stdin_to() {
  mkdir -p "$(dirname "$1")"
  cat >"$1"
}

setup_workspace() {
  TEST_WORKSPACE="$(new_workspace)"
}

cleanup_workspace() {
  rm -rf "$TEST_WORKSPACE"
}
