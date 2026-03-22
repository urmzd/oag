default: check

install-hooks:
    git config core.hooksPath .githooks

init: install-hooks
    rustup component add clippy rustfmt

install:
    cargo build --release -p oag-cli

build:
    cargo build --workspace

run *ARGS:
    cargo run -p oag-cli -- {{ARGS}}

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
    cargo publish -p oag-node-client --dry-run
    cargo publish -p oag-react-swr-client --dry-run
    cargo publish -p oag-fastapi-server --dry-run
    cargo publish -p oag-cli --dry-run

test-integration:
    cargo test --workspace

examples: install
    cd examples/petstore && ../../target/release/oag generate
    cd examples/sse-chat && ../../target/release/oag generate
    cd examples/anthropic-messages && ../../target/release/oag generate
    cd examples/petstore-polymorphic && ../../target/release/oag generate

record: install
    rm -rf /tmp/oag-demo && mkdir -p /tmp/oag-demo
    PATH="$(pwd)/target/release:$PATH" SPEC="$(pwd)/crates/oag-core/tests/fixtures/petstore-3.2.yaml" teasr showme

check: check-fmt lint test

ci: check-fmt lint build test
