# AGENTS.md

## Identity

You are an agent working on **oag** (OpenAPI Generator) — an OpenAPI 3.x code generator with a plugin-style architecture. It generates TypeScript/Node API clients, React/SWR hooks, and Python FastAPI server stubs from OpenAPI specs.

## Architecture

Rust workspace with five crates:

| Crate | Role |
|-------|------|
| `oag-core` | OpenAPI parser, intermediate representation (IR), transform pipeline, `CodeGenerator` trait |
| `oag-node-client` | TypeScript/Node API client generator (zero runtime deps) |
| `oag-react-swr-client` | React/SWR hooks generator (extends node-client) |
| `oag-fastapi-server` | Python FastAPI server generator with Pydantic v2 models |
| `oag-cli` | CLI entry point (`clap`) that orchestrates all generators |

```
oag-cli --> [oag-node-client, oag-react-swr-client, oag-fastapi-server] --> oag-core
```

Each generator implements the `CodeGenerator` trait:

```rust
pub trait CodeGenerator {
    fn id(&self) -> config::GeneratorId;
    fn generate(&self, ir: &ir::IrSpec, config: &config::GeneratorConfig) -> Result<Vec<GeneratedFile>, GeneratorError>;
}
```

## Key Files

- `crates/oag-cli/src/main.rs` — CLI entry point
- `crates/oag-core/src/` — IR, parser, config, transform pipeline
- `crates/oag-core/default-config.yaml` — Default `oag.yaml` config
- `examples/` — Working examples (petstore, sse-chat, anthropic-messages, petstore-polymorphic)

## Commands

| Task | Command |
|------|---------|
| Build | `just build` or `cargo build --workspace` |
| Test | `just test` or `cargo test --workspace` |
| Lint | `just lint` or `cargo clippy --workspace -- -D warnings` |
| Format | `just fmt` or `cargo fmt --all` |
| Check format | `just check-fmt` |
| Install binary | `just install` or `cargo build --release -p oag-cli` |
| Run CLI | `just run <ARGS>` or `cargo run -p oag-cli -- <ARGS>` |
| Generate examples | `just examples` |
| Full CI check | `just ci` (format + lint + build + test) |

## Code Style

- Rust 2024 edition, Apache-2.0 license
- `cargo fmt` and `cargo clippy -- -D warnings` enforced via `.githooks/`
- Snapshot testing with `insta` (YAML mode)
- Templates use `minijinja`, case conversion via `heck`
- Workspace version: all crates share `workspace.package.version`

## Adding a New Generator

1. Create a new crate under `crates/oag-<name>/`
2. Implement `CodeGenerator` trait from `oag-core`
3. Register the generator ID in `oag-core/src/config.rs` (`GeneratorId` enum)
4. Wire it into `oag-cli/src/main.rs`
5. Add an example under `examples/`
