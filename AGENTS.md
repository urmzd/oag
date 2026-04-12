# AGENTS.md

## Identity

You are an agent working on **oag** — an OpenAPI 3.x code generator with a template pack engine. Generates TypeScript/Node clients, React/SWR hooks, and Python FastAPI stubs.

## Architecture

Rust workspace with two crates and declarative template packs:

```
oag-cli  -->  oag-core (engine)
                 ├── .oag/packs/node-client/
                 ├── .oag/packs/react-swr-client/  (extends node-client)
                 └── .oag/packs/fastapi-server/
```

| Component | Role |
|-----------|------|
| `oag-core` | Parser, IR, transforms, template pack engine |
| `oag-cli` | CLI (`clap`), pack resolution, orchestration |
| `packs/` | Jinja2 templates + TOML manifests (embedded at compile time) |

Packs install locally to `.oag/packs/` in the project directory. Packs support inheritance (`extends` in `oag.pack.toml`).

Core engine API:

```rust
pub fn generate(ir: &IrSpec, config: &GeneratorConfig, pack: &TemplatePack)
    -> Result<Vec<GeneratedFile>, GeneratorError>
```

## CLI

```
oag generate [spec]              generate code (spec optional, defaults to oag.yaml)
oag validate <spec>              parse spec and report stats
oag inspect <spec>               dump IR as JSON
oag init [-p <pack>]             create oag.yaml, optionally install packs
oag check                        run linters/typecheckers on output
oag packs list|install|remove            manage packs in .oag/packs/
oag completions <shell>          shell completions
oag update                       self-update
```

## Key Files

- `crates/oag-cli/src/main.rs` — CLI entry point
- `crates/oag-core/src/engine/` — Template engine, pack resolution, type mapping
- `crates/oag-core/src/` — IR, parser, config, transforms
- `crates/oag-core/default-config.yaml` — Default `oag.yaml`
- `packs/*/oag.pack.toml` — Pack manifests
- `packs/*/templates/` — Jinja2 templates
- `examples/` — petstore, sse-chat

## Commands

| Task | Command |
|------|---------|
| Build | `just build` or `cargo build --workspace` |
| Test | `just test` or `cargo test --workspace` |
| Lint | `just lint` or `cargo clippy --workspace -- -D warnings` |
| Format | `just fmt` or `cargo fmt --all` |
| Install | `just install` or `cargo build --release -p oag-cli` |
| Run | `just run <ARGS>` or `cargo run -p oag-cli -- <ARGS>` |
| Examples | `just examples` |
| Full CI | `just ci` |

## Code Style

- Rust 2024 edition, Apache-2.0
- `cargo fmt` + `cargo clippy -- -D warnings` enforced via `.githooks/`
- Snapshot testing with `insta` (YAML mode)
- Templates use `minijinja`, case conversion via `heck`

## Adding a New Generator

1. Create `packs/<name>/` with `oag.pack.toml` and `templates/`
2. Test with `oag generate` using your pack ID in `oag.yaml`
3. Add an example under `examples/`

No Rust code needed — packs are fully declarative.
