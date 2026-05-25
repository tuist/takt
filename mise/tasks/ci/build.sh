#!/usr/bin/env bash
#MISE description="Build Takt with compiler warnings denied"
set -euo pipefail

export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-Dwarnings"

cargo build --locked --workspace --all-targets
