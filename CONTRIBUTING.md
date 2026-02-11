# Contributing

Thanks for your interest in contributing to `oag`.

## Prerequisites

- **Rust** (edition 2024) — install via [rustup](https://rustup.rs/)
- **Node.js 20+** — needed for integration compile tests
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
| `just test` | Run all workspace tests |
| `just build` | Build all crates |
| `just run <args>` | Run the CLI (e.g. `just run generate -i spec.yaml -o out`) |
| `just examples` | Rebuild the example output in `examples/` |
| `just record` | Record the demo GIF with [VHS](https://github.com/charmbracelet/vhs) |

## Project structure

```
crates/
  oag-core/         Core parser, IR types, and transform pipeline
  oag-typescript/   TypeScript client generator
  oag-react/        React/SWR hooks generator
  oag-cli/          CLI binary (oag)
examples/
  petstore/         TypeScript example (Petstore 3.2)
  sse-chat/         React + SSE example
tests/
  integration/      Integration compile tests (TypeScript/React)
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

1. Create a new crate under `crates/` (e.g. `oag-python`)
2. Depend on `oag-core`
3. Implement the `CodeGenerator` trait:

```rust
use oag_core::{CodeGenerator, GeneratedFile, ir::IrSpec};

pub struct PythonGenerator;

impl CodeGenerator for PythonGenerator {
    type Config = PythonConfig;
    type Error = PythonError;

    fn generate(
        &self,
        ir: &IrSpec,
        config: &Self::Config,
    ) -> Result<Vec<GeneratedFile>, Self::Error> {
        // ...
    }
}
```

4. Wire it into the CLI in `crates/oag-cli/src/main.rs`
5. Add the crate to the workspace `Cargo.toml` members

## Publishing

Publishing to crates.io is handled by CI via semantic-release on pushes to `main`. To do a local dry-run:

```sh
just publish
```

Crates are published in dependency order: `oag-core` -> `oag-typescript` -> `oag-react` -> `oag-cli`.
