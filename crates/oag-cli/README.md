# oag-cli

Command-line interface for `oag`.

Installs a single binary called `oag` that parses OpenAPI specs and generates typed client code.

## Install

```sh
cargo install oag-cli
```

## Commands

| Command | Description |
|---------|-------------|
| `oag generate` | Generate client code from an OpenAPI spec |
| `oag validate` | Validate an OpenAPI spec and report errors |
| `oag inspect` | Dump the parsed intermediate representation (YAML or JSON) |
| `oag init` | Create a `.urmzd.oag.yaml` config file in the current directory |
| `oag completions <shell>` | Generate shell completions (bash, zsh, fish, etc.) |

## Configuration

The CLI automatically loads `.urmzd.oag.yaml` from the current directory. You can override the input spec with `-i/--input`:

```sh
# Use config file
oag generate

# Override input spec
oag generate -i other-spec.yaml
```

The new config format uses a `generators` map instead of a `target` field. Each generator has its own output directory and settings. See the [root README](../../README.md#configuration) for the full configuration reference.

The old config format (with `target`, `output`, `output_options`, and `client` fields) is still supported for backward compatibility.

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, config, and `CodeGenerator` trait
- [`oag-node-client`](../oag-node-client/) — TypeScript/Node client generator
- [`oag-react-swr-client`](../oag-react-swr-client/) — React/SWR hooks generator
- [`oag-fastapi-server`](../oag-fastapi-server/) — Python FastAPI server generator

## Part of [oag](../../README.md)
