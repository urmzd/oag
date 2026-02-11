# oag

OpenAPI 3.x code generator with a plugin-style architecture supporting TypeScript, React, and Python FastAPI.

![demo](doc/demo.gif)

## Why oag?

OpenAPI 3.2 shipped but most generators haven't caught up. When you need to glue a frontend to a backend during a POC, you don't want to fight a generator that produces bloated code requiring heavy post-processing.

`oag` focuses on simplicity: one config file, one command, clean output.

- Parses OpenAPI 3.x specs with full `$ref` resolution
- Plugin-style architecture: enable only the generators you need
- **TypeScript/Node client** with zero runtime dependencies
- **React/SWR hooks** for queries, mutations, and SSE streaming
- **Python FastAPI server** with Pydantic v2 models
- First-class Server-Sent Events support via `AsyncGenerator` (TS) and `StreamingResponse` (Python)
- **Test generation** — pytest tests for FastAPI, vitest tests for TypeScript/React (opt-out via `scaffold.tests: false`)
- Scaffolds Biome + tsdown configuration for TypeScript projects
- Configurable naming strategies and operation aliases
- Three layout modes per generator: bundled, modular, or split

## Quick start

Install from crates.io:

```sh
cargo install oag-cli
```

Or build from source:

```sh
git clone https://github.com/urmzd/openapi-generator.git
cd openapi-generator
cargo install --path crates/oag-cli
```

Initialize a config file:

```sh
oag init
```

This creates `.urmzd.oag.yaml` in the current directory:

```yaml
input: openapi.yaml

naming:
  strategy: use_operation_id  # use_operation_id | use_route_based
  aliases: {}
    # createChatCompletion: chat
    # listModels: models

generators:
  node-client:
    output: src/generated/node
    layout: modular           # bundled | modular | split
    # split_by: tag           # operation | tag | route (only for split)
    # base_url: https://api.example.com
    # no_jsdoc: false
    scaffold:
      # package_name: my-api-client
      # repository: https://github.com/you/your-repo
      biome: true
      tsdown: true
      tests: true              # generate test files + dev dependencies

  # react-swr-client:
  #   output: src/generated/react
  #   layout: modular
  #   scaffold:
  #     biome: true
  #     tsdown: true

  # fastapi-server:
  #   output: src/generated/server
  #   layout: modular
```

Generate code:

```sh
oag generate
```

This will generate code for all configured generators. You can override the input spec:

```sh
oag generate -i other-spec.yaml
```

**Note**: The old config format (with `target`, `output`, `output_options`, and `client` fields) is still supported for backward compatibility.

## CLI reference

| Command | Description |
|---------|-------------|
| `generate` | Generate client code from an OpenAPI spec |
| `validate` | Validate an OpenAPI spec and report errors |
| `inspect` | Dump the parsed intermediate representation (YAML or JSON) |
| `init` | Create a `.urmzd.oag.yaml` config file |
| `completions` | Generate shell completions (bash, zsh, fish, etc.) |

Run `oag <command> --help` for detailed usage.

## Configuration

All options are set in `.urmzd.oag.yaml`. The CLI supports `-i/--input` to override the input spec path.

### Global options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `input` | `string` | `openapi.yaml` | Path to the OpenAPI spec (YAML or JSON) |
| `naming.strategy` | `string` | `use_operation_id` | How to derive function names: `use_operation_id` or `use_route_based` |
| `naming.aliases` | `map` | `{}` | Map of operationId to custom name overrides |

### Generators

The `generators` map configures which generators to run and their options. Each generator has its own output directory and settings.

**Available generators:**
- `node-client` — TypeScript/Node API client (zero dependencies)
- `react-swr-client` — React/SWR hooks (extends node-client)
- `fastapi-server` — Python FastAPI server stubs with Pydantic v2 models

### Generator options (node-client, react-swr-client, fastapi-server)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `output` | `string` | **required** | Output directory for this generator |
| `layout` | `string` | `modular` | Layout mode: `bundled` (single file), `modular` (separate files per concern), or `split` (separate files per operation group) |
| `split_by` | `string` | `tag` | Only for `split` layout: `operation`, `tag`, or `route` |
| `base_url` | `string` | *(from spec servers)* | Override the API base URL (TypeScript generators only) |
| `no_jsdoc` | `bool` | `false` | Disable JSDoc comments (TypeScript generators only) |
| `scaffold.package_name` | `string` | *(from spec title)* | Custom package name (TypeScript: npm, Python: pyproject.toml) |
| `scaffold.repository` | `string` | | Repository URL for package metadata |
| `scaffold.biome` | `bool` | `true` | Generate `biome.json` and auto-format output (TypeScript only) |
| `scaffold.tsdown` | `bool` | `true` | Generate `tsdown.config.ts` (TypeScript only) |
| `scaffold.tests` | `bool` | `true` | Generate test files and dev dependencies (vitest for TS, pytest for Python) |

### Layout modes

- **bundled** — Everything in a single file (e.g., `index.ts` or `main.py`)
- **modular** — Separate files per concern (e.g., `types.ts`, `client.ts`, `sse.ts`, `index.ts`)
- **split** — Separate files per operation group (e.g., `pets.ts`, `users.ts`, `orders.ts`)

When using `split` layout, specify `split_by`:
- `operation` — One file per operation
- `tag` — One file per OpenAPI tag (default)
- `route` — One file per route prefix

### Backward compatibility

The old config format (with `target`, `output`, `output_options`, and `client` fields) is still supported and automatically converted to the new format.

## Architecture

```
oag-cli  -->  [oag-node-client, oag-react-swr-client, oag-fastapi-server]  -->  oag-core
```

The workspace uses a plugin-style architecture with five crates:

| Crate | Role |
|-------|------|
| [`oag-core`](crates/oag-core/) | OpenAPI parser, intermediate representation, transform pipeline, and `CodeGenerator` trait |
| [`oag-node-client`](crates/oag-node-client/) | TypeScript/Node API client generator (zero dependencies) |
| [`oag-react-swr-client`](crates/oag-react-swr-client/) | React/SWR hooks generator (extends node-client) |
| [`oag-fastapi-server`](crates/oag-fastapi-server/) | Python FastAPI server generator with Pydantic v2 models |
| [`oag-cli`](crates/oag-cli/) | Command-line interface that orchestrates all generators |

`oag-core` defines the `CodeGenerator` trait:

```rust
pub trait CodeGenerator {
    fn id(&self) -> config::GeneratorId;
    fn generate(
        &self,
        ir: &ir::IrSpec,
        config: &config::GeneratorConfig,
    ) -> Result<Vec<GeneratedFile>, GeneratorError>;
}
```

Each generator implements this trait with a unique ID (`node-client`, `react-swr-client`, or `fastapi-server`). The CLI loops over the configured generators in `.urmzd.oag.yaml` and invokes each one.

## Examples

Working examples with generated output are in the [`examples/`](examples/) directory:

- **[`petstore`](examples/petstore/)** — Node client and React client generated from the Petstore 3.2 spec
- **[`sse-chat`](examples/sse-chat/)** — Node client and React hooks with SSE streaming for a chat API

Each example has its own `.urmzd.oag.yaml` and a `generated/` directory with separate subdirectories for each generator:
- `generated/node/` — TypeScript/Node client
- `generated/react/` — React/SWR hooks

Regenerate them with:

```sh
just examples
```

## License

[Apache-2.0](https://opensource.org/licenses/Apache-2.0)
