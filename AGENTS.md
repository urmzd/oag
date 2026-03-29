# AGENTS.md

## Identity

You are an agent working on **oag** (OpenAPI Generator) — an OpenAPI 3.x code generator powered by a template pack engine. It generates TypeScript/Node API clients, React/SWR hooks, and Python FastAPI server stubs from OpenAPI specs.

## Architecture

Rust workspace with two crates and a set of declarative template packs:

| Component | Role |
|-----------|------|
| `oag-core` | OpenAPI parser, intermediate representation (IR), transform pipeline, and template pack engine |
| `oag-cli` | CLI entry point (`clap`) that resolves packs and orchestrates generation |
| `packs/` | Declarative template packs — Jinja2 templates + TOML manifests |

```
oag-cli  -->  oag-core (engine + packs)
                 ├── packs/node-client/
                 ├── packs/react-swr-client/  (extends node-client)
                 └── packs/fastapi-server/
```

Generators are defined as **template packs** rather than compiled Rust crates. Each pack contains a `pack.toml` manifest (metadata, type mappings, layouts, scaffold config) and `templates/` directory with Jinja2 `.j2` files. Packs support inheritance (`extends` in `pack.toml`). Built-in packs are embedded in the binary at compile time via `include_dir!`.

The core engine API:

```rust
pub fn generate(
    ir: &IrSpec,
    config: &GeneratorConfig,
    pack: &TemplatePack,
) -> Result<Vec<GeneratedFile>, GeneratorError>
```

## Key Files

- `crates/oag-cli/src/main.rs` — CLI entry point
- `crates/oag-core/src/engine/` — Template pack engine (rendering, context, pack resolution, type mapping)
- `crates/oag-core/src/` — IR, parser, config, transform pipeline
- `crates/oag-core/default-config.yaml` — Default `oag.yaml` config
- `packs/*/pack.toml` — Template pack manifests
- `packs/*/templates/` — Jinja2 templates
- `examples/` — Working examples (petstore, sse-chat)

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

1. Create a new directory under `packs/<name>/`
2. Write a `pack.toml` manifest (see existing packs for reference)
3. Add Jinja2 templates in `packs/<name>/templates/`
4. Test with `oag generate` using your new pack ID in `oag.yaml`
5. Add an example under `examples/`

No Rust code changes are needed for new generators — packs are fully declarative.
