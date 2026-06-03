#!/usr/bin/env bash
# Pre-release validation — regenerate every example from the working-tree packs
# and run each pack's validators. Must pass before a release can be tagged, and
# runs in CI on every pull request so breakage is caught before it reaches main.
set -euo pipefail

echo "==> Building oag..."
cargo build --release -p oag
bin="$PWD/target/release/oag"

echo "==> Regenerating examples (with working-tree packs)..."
for dir in examples/*/; do
  echo "    generate: $dir"
  (
    cd "$dir"
    # Install the working-tree packs so generation reflects local changes.
    # Without this, `oag generate` would download packs from GitHub @main and
    # ignore uncommitted pack edits. Installs land in the gitignored .oag/ dir,
    # so they never show up in the diff check below. Generators only use the
    # packs their oag.yaml references, so installing all of them is harmless.
    for pack in ../../packs/*/; do
      "$bin" packs install "$pack" >/dev/null
    done
    "$bin" generate
  )
done

echo "==> Validating generated output..."
for dir in examples/*/; do
  echo "    check: $dir"
  (cd "$dir" && "$bin" check)
done

echo "==> Checking for uncommitted changes after regeneration..."
if ! git diff --quiet; then
  echo "ERROR: Generated files are out of date. Run 'just examples' and commit the result."
  git diff --stat
  exit 1
fi

echo "==> Pre-release checks passed."
