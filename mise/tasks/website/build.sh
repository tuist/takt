#!/usr/bin/env bash
#MISE description="Build the Takt marketing website"
set -euo pipefail

cd "$MISE_PROJECT_ROOT/website"

aube install --frozen-lockfile
aube run build
