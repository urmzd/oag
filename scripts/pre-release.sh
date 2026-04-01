#!/usr/bin/env bash
# Pre-release validation — must pass before a release can be tagged.
set -euo pipefail

echo "==> Running CI checks..."
just ci

echo "==> Regenerating examples..."
just examples

echo "==> Typechecking generated node clients..."
for dir in examples/*/generated/node; do
  if [ -f "$dir/tsconfig.json" ]; then
    echo "    typecheck: $dir"
    (cd "$dir" && npx tsc --noEmit)
  fi
done

echo "==> Checking for uncommitted changes after regeneration..."
if ! git diff --quiet; then
  echo "ERROR: Generated files are out of date. Commit the regenerated output first."
  git diff --stat
  exit 1
fi

echo "==> Pre-release checks passed."
