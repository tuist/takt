#!/usr/bin/env bash
#MISE description="Check Takt formatting"
set -euo pipefail

cargo fmt --all --check
