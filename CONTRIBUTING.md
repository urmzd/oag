# Contributing

Thanks for your interest in contributing to `oag`.

## Prerequisites

- **Rust** (edition 2024) — install via [rustup](https://rustup.rs/)
- **Node.js 24+** *(optional)* — only needed for `#[ignore]`-marked integration compile tests
- **[just](https://github.com/casey/just)** — command runner

## Setup

```sh
git clone https://github.com/urmzd/oag.git
cd oag
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
| `just build` | Build all crates |
| `just run <args>` | Run the CLI (e.g. `just run generate -i spec.yaml`) |
| `just examples` | Rebuild the example output in `examples/` |
| `just record` | Record the demo GIF with [teasr](https://github.com/urmzd/teasr) |

## Project structure

```
crates/
  oag-core/              Core parser, IR types, transform pipeline, and template pack engine
  oag-cli/               CLI binary (oag)
packs/
  node-client/           TypeScript/Node client template pack
  react-swr-client/      React/SWR hooks template pack (extends node-client)
  fastapi-server/        Python FastAPI server template pack (Pydantic v2)
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

Generators are now **template packs** — no Rust code changes needed.

1. Create a new directory under `packs/` (e.g., `packs/go-client/`)
2. Write a `oag.pack.toml` manifest defining metadata, type mappings, layouts, and scaffold config (see existing packs for reference)
3. Add Jinja2 templates in `packs/go-client/templates/` (use `.j2` extension)
4. Use `extends` in `oag.pack.toml` if your pack should inherit from an existing pack
5. Add your pack ID to `oag.yaml` under `generators:` and test with `oag generate`
6. Add an example under `examples/`
7. Add integration tests in `tests/integration/`

To install a custom pack without modifying the source:

```sh
oag templates install /path/to/your/pack
```

## Publishing

Publishing to crates.io is handled by CI via semantic-release on pushes to `main`. To do a local dry-run:

```sh
just publish
```

Crates are published in dependency order:
1. `oag-core` (foundation — includes template pack engine)
2. `oag-cli` (depends on core, embeds built-in packs)
