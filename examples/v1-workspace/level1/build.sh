#!/usr/bin/env bash
set -euo pipefail

workspace="${1:-examples/v1-workspace}"
printf 'checking %s\n' "$workspace"
find "$workspace" -maxdepth 5 -type f | sort
