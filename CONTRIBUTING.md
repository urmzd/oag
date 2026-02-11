# Contributing

Thanks for your interest in contributing to `oag`.

## Prerequisites

- **Rust** (edition 2024) — install via [rustup](https://rustup.rs/)
- **Node.js 20+** *(optional)* — only needed for `#[ignore]`-marked integration compile tests
- **[just](https://github.com/casey/just)** — command runner

## Setup

```sh
git clone https://github.com/urmzd/openapi-generator.git
cd openapi-generator
just init
```

This installs git hooks and adds the `clippy` and `rustfmt` components.

## Development workflow

| Command | What it does |
|---------|-------------|
| `just check` | Run format check, clippy, and tests (the default target) |
| `just fmt` | Format all code |
| `just lint` | Run clippy with `-D warnings` |
| `just test` | Run all workspace tests (excluding integration tests) |
| `just test-all` | Run all tests including integration tests (requires test servers) |
| `just build` | Build all crates |
| `just run <args>` | Run the CLI (e.g. `just run generate -i spec.yaml`) |
| `just examples` | Rebuild the example output in `examples/` |
| `just record` | Record the demo GIF with [VHS](https://github.com/charmbracelet/vhs) |

## Project structure

```
crates/
  oag-core/              Core parser, IR types, transform pipeline, and CodeGenerator trait
  oag-node-client/       TypeScript/Node client generator (zero deps)
  oag-react-swr-client/  React/SWR hooks generator (extends node-client)
  oag-fastapi-server/    Python FastAPI server generator (Pydantic v2)
  oag-cli/               CLI binary (oag)
examples/
  petstore/              Node client + React client examples (Petstore 3.2)
  sse-chat/              Node client + React + SSE streaming examples
tests/
  integration/           Integration tests with mock Axum servers (marked #[ignore])
```

## Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) as defined in `.urmzd.sr.yml`:

| Prefix | Bump | Section |
|--------|------|---------|
| `feat` | minor | Features |
| `fix` | patch | Bug Fixes |
| `perf` | patch | Performance |
| `docs` | — | Documentation |
| `refactor` | — | Refactoring |
| `revert` | — | Reverts |
| `chore` | — | *(hidden)* |
| `ci` | — | *(hidden)* |
| `test` | — | *(hidden)* |
| `build` | — | *(hidden)* |
| `style` | — | *(hidden)* |

Format: `type(scope): description`

Breaking changes: append `!` after the type/scope (e.g. `feat!: drop Node 18 support`).

## Pull requests

- Fill out the [PR template](.github/pull_request_template.md)
- Make sure CI passes:
  - `cargo clippy --workspace -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
- Keep PRs focused — one logical change per PR

## Adding a new generator

1. Create a new crate under `crates/` (e.g., `oag-go-client`)
2. Add it to the workspace `Cargo.toml` members
3. Depend on `oag-core` in the new crate's `Cargo.toml`
4. Add a new variant to `GeneratorId` in `crates/oag-core/src/config.rs`
5. Implement the `CodeGenerator` trait:

```rust
use oag_core::{CodeGenerator, GeneratedFile, GeneratorError, config, ir};

pub struct GoClientGenerator;

impl CodeGenerator for GoClientGenerator {
    fn id(&self) -> config::GeneratorId {
        config::GeneratorId::GoClient // (add this variant to the enum)
    }

    fn generate(
        &self,
        ir: &ir::IrSpec,
        config: &config::GeneratorConfig,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        // ...
    }
}
```

6. Register the generator in `crates/oag-cli/src/main.rs`:
   - Import the generator
   - Add it to the generator registry
   - Handle its `GeneratorId` in the match arms

7. Add documentation: `crates/oag-go-client/README.md`
8. Add integration tests in `tests/integration/`
9. Update the `just publish` command in `justfile` to include the new crate

## Publishing

Publishing to crates.io is handled by CI via semantic-release on pushes to `main`. To do a local dry-run:

```sh
just publish
```

Crates are published in dependency order:
1. `oag-core` (foundation)
2. `oag-node-client` (depends on core)
3. `oag-react-swr-client` (depends on core)
4. `oag-fastapi-server` (depends on core)
5. `oag-cli` (depends on all generators)
