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

The CLI automatically loads `.urmzd.oag.yaml` from the current directory. CLI flags override config values:

```sh
# Use config file
oag generate

# Override specific options
oag generate -i other-spec.yaml -o dist/api -t react --base-url https://api.example.com
```

See the [root README](../../README.md#configuration) for the full configuration reference.

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, and config
- [`oag-typescript`](../oag-typescript/) — TypeScript generator
- [`oag-react`](../oag-react/) — React generator

## Part of [oag](../../README.md)
