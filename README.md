# oag

OpenAPI 3.x code generator for TypeScript and React.

![demo](doc/demo.gif)

## Why oag?

OpenAPI 3.2 shipped but most generators haven't caught up. When you need to glue a frontend to a backend during a POC, you don't want to fight a generator that produces bloated code requiring heavy post-processing.

`oag` focuses on simplicity: one config file, one command, clean output.

- Parses OpenAPI 3.x specs with full `$ref` resolution
- Generates a typed TypeScript client with **zero runtime dependencies**
- Generates React/SWR hooks (query, mutation, SSE streaming)
- First-class Server-Sent Events support via `AsyncGenerator`
- Scaffolds Biome + tsdown configuration out of the box
- Configurable naming strategies and operation aliases

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
output: src/generated
target: all           # typescript | react | all

naming:
  strategy: use_operation_id  # use_operation_id | use_route_based
  aliases: {}
    # createChatCompletion: chat
    # listModels: models

output_options:
  layout: single        # single | split (split = typescript/ + react/ subdirs)
  index: true           # generate index.ts barrel exports
  biome: true           # generate biome.json and format output
  tsdown: true          # generate tsdown.config.ts
  # package_name: my-api-client
  # repository: https://github.com/you/your-repo

client:
  # base_url: https://api.example.com
  no_jsdoc: false
```

Generate code:

```sh
oag generate
```

Or pass everything via flags:

```sh
oag generate -i spec.yaml -o src/generated -t typescript
```

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

All options can be set in `.urmzd.oag.yaml` or overridden via CLI flags.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `input` | `string` | `openapi.yaml` | Path to the OpenAPI spec (YAML or JSON) |
| `output` | `string` | `src/generated` | Output directory for generated code |
| `target` | `string` | `all` | Generator target: `typescript`, `react`, or `all` |
| `naming.strategy` | `string` | `use_operation_id` | How to derive function names: `use_operation_id` or `use_route_based` |
| `naming.aliases` | `map` | `{}` | Map of operationId to custom name overrides |
| `output_options.layout` | `string` | `single` | `single` puts everything in one directory; `split` creates `typescript/` and `react/` subdirs |
| `output_options.index` | `bool` | `true` | Generate `index.ts` barrel exports |
| `output_options.biome` | `bool` | `true` | Generate `biome.json` |
| `output_options.tsdown` | `bool` | `true` | Generate `tsdown.config.ts` |
| `output_options.package_name` | `string` | *(from spec title)* | Custom npm package name |
| `output_options.repository` | `string` | | Repository URL for `package.json` |
| `client.base_url` | `string` | *(from spec servers)* | Override the API base URL |
| `client.no_jsdoc` | `bool` | `false` | Disable JSDoc comments in generated code |

## Architecture

```
oag-cli  -->  oag-react  -->  oag-typescript  -->  oag-core
```

The workspace is split into four crates, each with a single responsibility:

| Crate | Role |
|-------|------|
| [`oag-core`](crates/oag-core/) | OpenAPI parser, intermediate representation, and transform pipeline |
| [`oag-typescript`](crates/oag-typescript/) | TypeScript client code generator |
| [`oag-react`](crates/oag-react/) | React/SWR hooks generator (extends TypeScript) |
| [`oag-cli`](crates/oag-cli/) | Command-line interface |

`oag-core` defines the `CodeGenerator` trait that both `oag-typescript` and `oag-react` implement. The CLI wires everything together.

## Examples

Working examples with generated output are in the [`examples/`](examples/) directory:

- **[`petstore`](examples/petstore/)** — TypeScript client generated from the Petstore 3.2 spec
- **[`sse-chat`](examples/sse-chat/)** — React hooks with SSE streaming for a chat API

Each example has its own `.urmzd.oag.yaml` and a `generated/` directory with the output. Regenerate them with:

```sh
just examples
```

## License

[MIT](https://opensource.org/licenses/MIT)
