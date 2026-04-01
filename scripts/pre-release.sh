#!/usr/bin/env bash
# Pre-release validation — must pass before a release can be tagged.
set -euo pipefail

echo "==> Regenerating examples..."
cargo build --release -p oag
for dir in examples/*/; do
  echo "    generate: $dir"
  (cd "$dir" && ../../target/release/oag generate)
done

echo "==> Validating generated output..."
for dir in examples/*/; do
  echo "    check: $dir"
  (cd "$dir" && ../../target/release/oag check)
done

echo "==> Checking for uncommitted changes after regeneration..."
if ! git diff --quiet; then
  echo "ERROR: Generated files are out of date. Commit the regenerated output first."
  git diff --stat
  exit 1
fi

echo "==> Pre-release checks passed."
