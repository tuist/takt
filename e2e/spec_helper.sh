TAKT_ROOT="${SHELLSPEC_PROJECT_ROOT:-$(pwd)}"

run_takt() {
  mise exec -- cargo run --quiet --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
}

run_takt_in() {
  local dir="$1"
  shift
  (
    cd "$dir" &&
      mise exec -- cargo run --quiet --manifest-path "$TAKT_ROOT/Cargo.toml" -- "$@"
  )
}

new_workspace() {
  mktemp -d "${TMPDIR:-/tmp}/takt-e2e.XXXXXX"
}

cat_file() {
  cat "$1"
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
