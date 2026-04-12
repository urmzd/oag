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
oag templates list               list installed and available packs
oag templates install <path>     install a pack from a local directory
oag templates install --id <id>  download a pack from GitHub
oag templates remove <id>        remove a pack
oag templates path               print the packs directory (.oag/packs/)
oag completions <shell>          generate shell completions (bash, zsh, fish, powershell, elvish)
oag update                       self-update to latest release
oag version                      print version
```

Packs are stored in `.oag/packs/` relative to your project root. Commit them, gitignore them, or customize them — your call.

## Configuration

`oag init` creates `oag.yaml`:

<!-- embed-src src="crates/oag-core/default-config.yaml" fence="yaml" -->
```yaml
# oag configuration — https://github.com/urmzd/oag
#
# This file is loaded automatically from the current directory when running `oag generate`.
# You can override the input spec with: oag generate other-spec.yaml
#
# Full reference: https://github.com/urmzd/oag#configuration

# ---------------------------------------------------------------------------
# Input
# ---------------------------------------------------------------------------
# Path to your OpenAPI 3.x spec (YAML or JSON), relative to this config file.
input: openapi.yaml

# ---------------------------------------------------------------------------
# Naming
# ---------------------------------------------------------------------------
# Controls how operation names (function/method names) are derived.
naming:
  # Strategy for deriving operation names:
  #   use_operation_id  — use the operationId field from the spec (default)
  #   use_route_based   — derive from HTTP method + path (e.g., GET /pets → getPets)
  strategy: use_operation_id

  # Custom aliases to rename specific operations. Applied after the naming strategy.
  # Keys are the resolved operation name, values are the desired alias.
  aliases: {}
    # createChatCompletion: chat     # operationId → custom name
    # listModels: models

# ---------------------------------------------------------------------------
# Generators
# ---------------------------------------------------------------------------
# Each key is a generator ID. Only generators listed here will run.
# Available generators:
#   node-client       — TypeScript/Node API client (zero runtime dependencies)
#   react-swr-client  — React/SWR hooks (extends node-client with hooks + context provider)
#   fastapi-server    — Python FastAPI server stubs with Pydantic v2 models
generators:
  node-client:
    # Directory where generated files are written. Created automatically.
    output: src/generated/node

    # How files are organized:
    #   bundled  — single file (src/index.ts)
    #   modular  — separate files per concern: types.ts, client.ts, sse.ts, index.ts (default)
    #   split    — separate files per operation group (see split_by)
    layout: modular

    # Only used with layout: split. Controls how operations are grouped into files:
    #   tag        — one file per OpenAPI tag (default)
    #   operation  — one file per operation
    #   route      — one file per route prefix
    # split_by: tag

    # Override the API base URL instead of reading from the spec's servers array.
    # Useful when the spec omits a server or you need a different URL for development.
    # base_url: https://api.example.com

    # Set to true to disable JSDoc comments on generated types and methods.
    # no_jsdoc: false

    # Subdirectory within output for generated source files.
    # Scaffold files (package.json, tsconfig, etc.) always stay at the output root.
    # Set to "" to place source files directly at the output root.
    # source_dir: src

    # Set scaffold to false to disable all scaffolding (for existing projects).
    # scaffold: false
    #
    # Scaffold controls which project configuration files are generated alongside
    # the source code. Set individual tools to false to disable them.
    scaffold:
      # NPM package name. Defaults to a slugified version of the spec's info.title.
      # package_name: my-api-client

      # Repository URL included in package.json.
      # repository: https://github.com/you/your-repo

      # Set to true to skip scaffold files (package.json, tsconfig, biome, tsdown)
      # but still emit a root index.ts re-export. Useful when adding generated code
      # to an existing project with its own build configuration.
      # existing_repo: false

      # Code formatter. Generates biome.json and auto-formats after generation.
      # Set to false to disable.
      formatter: biome        # biome | false

      # Test runner. Generates vitest test files and adds vitest to package.json.
      # Set to false to disable test generation.
      test_runner: vitest     # vitest | false

      # Bundler. Generates tsdown.config.ts for building distributable packages.
      # Set to false to disable.
      bundler: tsdown         # tsdown | false

      # Extra dev dependencies to include in package.json devDependencies.
      # Each key is a package name, each value is a version specifier.
      # extra_dev_dependencies:
      #   "@testing-library/react": "^16.0"
      #   msw: "^2.0"

  # react-swr-client:
  #   output: src/generated/react
  #   layout: modular           # only modular is supported for react-swr-client
  #   # base_url: https://api.example.com
  #   # no_jsdoc: false
  #   # source_dir: src
  #   scaffold:
  #     # package_name: my-react-client
  #     formatter: biome        # biome | false
  #     test_runner: vitest     # vitest | false
  #     bundler: tsdown         # tsdown | false

  # fastapi-server:
  #   output: src/generated/server
  #   layout: modular           # only modular is supported for fastapi-server
  #   scaffold:
  #     # package_name: my_api_server
  #     formatter: ruff         # ruff | false — auto-formats and lints after generation
  #     test_runner: pytest     # pytest | false — generates pytest tests with async httpx client
  #     # extra_dev_dependencies:
  #     #   factory-boy: ">=3.3"
  #     #   httpx: ">=0.27"
```
<!-- /embed-src -->

### Generator options

| Key | Description |
|-----|-------------|
| `output` | Output directory (required) |
| `layout` | `bundled`, `modular` (default), or `split` |
| `split_by` | For split layout: `tag` (default), `operation`, or `route` |
| `base_url` | Override API base URL |
| `no_jsdoc` | Disable JSDoc comments (TS only) |
| `source_dir` | Subdirectory for source files, default `src` |
| `scaffold` | Set to `false` to disable, or configure `formatter`, `test_runner`, `bundler` |

### Scaffold behavior

Scaffold files (`package.json`, `tsconfig.json`, etc.) are **write-once** — they're only created if they don't exist, so your customizations survive regeneration. Source files are always overwritten. Use `--force-scaffold` to reset them.

### Layout modes

- **bundled** — single file
- **modular** — separate files per concern (types, client, sse, index)
- **split** — separate files per operation group (by tag, operation, or route)

## Template packs

Packs live in `.oag/packs/` inside your project. Each pack is a directory with:

- `oag.pack.toml` — manifest (metadata, type mappings, layouts, formatters)
- `templates/` — Jinja2 `.j2` templates

Packs support inheritance (`extends` in the manifest). `react-swr-client` extends `node-client` — it inherits all base templates and adds React hooks on top.

To customize a built-in pack, install it locally and edit:

```sh
oag templates install --id node-client
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
