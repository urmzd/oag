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
        # The binary embeds the working-tree packs; installing them from the
        # source path as well replaces any stale copy left in the example's
        # .oag/packs/, which would otherwise take precedence.
        "$bin" packs install "../../packs/$pack" >/dev/null
      done
      "$bin" generate --force-scaffold
      popd >/dev/null
    }
    gen petstore node-client react-swr-client
    gen sse-chat node-client react-swr-client
    gen anthropic-messages node-client react-swr-client
    gen petstore-polymorphic node-client react-swr-client
    gen literal-default node-client react-swr-client fastapi-server
    gen event-stream node-client react-swr-client

record: install
    rm -rf /tmp/oag-demo && mkdir -p /tmp/oag-demo
    PATH="$(pwd)/target/release:$PATH" SPEC="$(pwd)/crates/oag-core/tests/fixtures/petstore-3.2.yaml" teasr showme

check: check-fmt lint test

ci: check-fmt lint build test
