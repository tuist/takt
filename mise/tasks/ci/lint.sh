#!/usr/bin/env bash
#MISE description="Lint Takt with warnings denied"
set -euo pipefail

export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-Dwarnings"

cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
