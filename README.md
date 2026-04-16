<p align="center">
  <h1 align="center">oag</h1>
  <p align="center">
    OpenAPI 3.x code generator. TypeScript, React, Python FastAPI.
    <br /><br />
    <a href="https://github.com/urmzd/oag/releases">Download</a>
    &middot;
    <a href="https://github.com/urmzd/oag/issues">Report Bug</a>
    &middot;
    <a href="https://github.com/urmzd/oag/tree/main/examples">Examples</a>
  </p>
</p>

<p align="center">
  <a href="https://github.com/urmzd/oag/actions/workflows/ci.yml"><img src="https://github.com/urmzd/oag/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/oag"><img src="https://img.shields.io/crates/v/oag" alt="crates.io"></a>
</p>

![demo](doc/demo.gif)

## Why oag?

Most OpenAPI generators produce bloated output that needs heavy post-processing. `oag` generates clean, readable code from one config file with one command.

- Parses OpenAPI 3.x specs with full `$ref` resolution
- Template pack engine — generators are Jinja2 templates, no Rust code needed
- Built-in packs: `node-client`, `react-swr-client`, `fastapi-server`
- First-class SSE support (`AsyncGenerator` in TS, `StreamingResponse` in Python)
- Packs install locally to `.oag/packs/` — version them, share them, customize them

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/urmzd/oag/main/install.sh | sh
```

Or via cargo:

```sh
cargo install oag
```

Or download binaries from [releases](https://github.com/urmzd/oag/releases/latest).

## Quick start

```sh
oag init -p node-client          # creates oag.yaml, installs pack to .oag/packs/
oag generate                     # generates code
```

## CLI

```
oag generate [spec]              generate code (spec is optional, defaults to oag.yaml input)
oag generate [spec] --force-scaffold   regenerate scaffold files too
oag validate <spec>              parse an OpenAPI spec and report stats
oag inspect <spec>               dump the parsed IR as JSON
oag init                         create oag.yaml in the current directory
oag init -p <pack> [-p <pack>]   also install packs
oag init --force                 overwrite existing oag.yaml
oag check                        run linters/typecheckers on generated output
oag packs list                   list installed and available packs
oag packs install <path>         install a pack from a local directory
oag packs install --id <id>      download a pack from GitHub
oag packs remove <id>            remove a pack
oag completions <shell>          generate shell completions (bash, zsh, fish, powershell, elvish)
oag update                       self-update to latest release
oag version                      print version
```

Packs are stored in `.oag/packs/` relative to your project root. Commit them, gitignore them, or customize them — your call.

## Configuration

`oag init` creates `oag.yaml`:

<!-- fsrc src="crates/oag-core/default-config.yaml" fence="yaml" -->
```yaml
# oag configuration — https://github.com/urmzd/oag
#
# Loaded automatically when running `oag generate`.
# Override the spec with: oag generate other-spec.yaml

input: openapi.yaml

generators:
  node-client:
    output: src/generated

  # react-swr-client:
  #   output: src/generated/react

  # fastapi-server:
  #   output: src/generated/server

# --- Standalone package (pnpm workspace, publishable client) ---
#
# To generate a standalone package instead of drop-in files, add scaffold
# and set source_dir so the package has its own src/ with build tooling:
#
#   node-client:
#     output: packages/api-client
#     source_dir: src
#     scaffold:
#       package_name: "@myorg/api-client"
#       formatter: biome        # biome | false
#       test_runner: vitest     # vitest | false
#       bundler: tsdown         # tsdown | false

# --- Other options ---
#
# naming:
#   strategy: use_operation_id  # or use_route_based
#   aliases:
#     createChatCompletion: chat
#
# Generator options:
#   layout: modular             # bundled | modular | split
#   split_by: tag               # tag | operation | route (only with layout: split)
#   base_url: https://api.example.com
#   no_jsdoc: false
```
<!-- /fsrc -->

By default, oag generates source files directly into the output directory — no `package.json`, no build tooling, no extra nesting. Just drop-in files you import from your existing project:

```
src/generated/
  types.ts
  client.ts
  guards.ts
  sse.ts
  index.ts
```

### Standalone package mode

To generate a publishable package (e.g., for a pnpm workspace), add `scaffold` and `source_dir`:

```yaml
generators:
  node-client:
    output: packages/api-client
    source_dir: src
    scaffold:
      package_name: "@myorg/api-client"
```

This produces a self-contained package with `package.json`, `tsconfig.json`, biome, tsdown, and vitest pre-configured:

```
packages/api-client/
  package.json
  tsconfig.json
  biome.json
  tsdown.config.ts
  src/
    types.ts
    client.ts
    ...
```

Scaffold files are **write-once** — they're only created if they don't exist, so your customizations survive regeneration. Use `--force-scaffold` to reset them.

### Layout modes

- **modular** (default) — separate files per concern (types, client, sse, index)
- **bundled** — everything in a single file
- **split** — separate files per operation group (by `tag`, `operation`, or `route`)

## Template packs

Packs live in `.oag/packs/` inside your project. Each pack is a directory with:

- `oag.pack.toml` — manifest (metadata, type mappings, layouts, formatters)
- `templates/` — Jinja2 `.j2` templates

Packs support inheritance (`extends` in the manifest). `react-swr-client` extends `node-client` — it inherits all base templates and adds React hooks on top.

To customize a built-in pack, install it locally and edit:

```sh
oag packs install --id node-client
# edit .oag/packs/node-client/templates/*.j2
oag generate
```

## Examples

See [`examples/`](examples/) for working projects:

- **[`petstore`](examples/petstore/)** — Node + React clients from the Petstore 3.2 spec
- **[`sse-chat`](examples/sse-chat/)** — SSE streaming with Node + React hooks

Regenerate with `just examples`.

## Architecture

```
oag-cli  -->  oag-core (engine)
                 ├── .oag/packs/node-client/
                 ├── .oag/packs/react-swr-client/  (extends node-client)
                 └── .oag/packs/fastapi-server/
```

| Component | Role |
|-----------|------|
| [`oag-core`](crates/oag-core/) | Parser, IR, transforms, template engine |
| [`oag-cli`](crates/oag-cli/) | CLI (clap), pack resolution, orchestration |
| [`packs/`](packs/) | Built-in template packs (embedded at compile time) |

## Agent Skill

This project ships an [Agent Skill](https://github.com/vercel-labs/skills) for Claude Code, Cursor, and other agents. See [`skills/`](skills/).
