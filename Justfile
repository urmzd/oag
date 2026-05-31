default: check

install-hooks:
    git config core.hooksPath .githooks

init: install-hooks
    rustup component add clippy rustfmt

install:
    cargo build --release -p oag

build:
    cargo build --workspace

run *ARGS:
    cargo run -p oag -- {{ARGS}}

test:
    cargo test --workspace

lint:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

check-fmt:
    cargo fmt --all -- --check

publish:
    cargo publish -p oag-core --dry-run
    cargo publish -p oag --dry-run

test-integration:
    cargo test --workspace

examples: install
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$PWD/target/release/oag"
    gen() {
      local dir="$1"; shift
      pushd "examples/$dir" >/dev/null
      for pack in "$@"; do
        # Install the working-tree pack so generation reflects local changes.
        # Pass only the source path (no --id): `packs install <path>` copies the
        # local pack, whereas `--id` would download it from GitHub @main and the
        # working-tree changes would be ignored. The id comes from oag.pack.toml.
        "$bin" packs install "../../packs/$pack" >/dev/null
      done
      "$bin" generate
      popd >/dev/null
    }
    gen petstore node-client react-swr-client
    gen sse-chat node-client react-swr-client
    gen anthropic-messages node-client react-swr-client
    gen petstore-polymorphic node-client react-swr-client
    gen literal-default node-client react-swr-client fastapi-server

record: install
    rm -rf /tmp/oag-demo && mkdir -p /tmp/oag-demo
    PATH="$(pwd)/target/release:$PATH" SPEC="$(pwd)/crates/oag-core/tests/fixtures/petstore-3.2.yaml" teasr showme

check: check-fmt lint test

ci: check-fmt lint build test
