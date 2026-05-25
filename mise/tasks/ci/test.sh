#!/usr/bin/env bash
#MISE description="Run Takt tests with compiler warnings denied"
set -euo pipefail

export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-Dwarnings"

cargo test --locked --workspace
