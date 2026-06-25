#!/usr/bin/env bash
# Count lines of user code — Rust, TOML, JSON schemas, shell scripts, Makefile
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "Counting lines of code in ${ROOT}..."
find "$ROOT" \
  -type f \
  \( -name "*.rs" -o -name "*.toml" -o -name "*.sh" -o -name "*.json" -o -name "Makefile" \) \
  -not -path "*/.git/*" \
  -not -path "*/target/*" \
  -not -name "Cargo.lock" \
  | sort \
  | xargs wc -l \
  | tail -1
