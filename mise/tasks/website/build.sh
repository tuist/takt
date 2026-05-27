#!/usr/bin/env bash
#MISE description="Build the Takt marketing website"
set -euo pipefail

cd "$(dirname "$0")/../../../website"

aube install --frozen-lockfile
aube run build
