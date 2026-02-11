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
    cargo publish -p oag-typescript --dry-run
    cargo publish -p oag-react --dry-run
    cargo publish -p oag-cli --dry-run

examples: install
    cd examples/petstore && ../../target/release/oag generate
    cd examples/sse-chat && ../../target/release/oag generate

check: check-fmt lint test
