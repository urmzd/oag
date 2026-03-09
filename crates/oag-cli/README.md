# oag-cli

Command-line interface for `oag` — an OpenAPI 3.x code generator.

Installs a single binary called `oag` that parses OpenAPI specs and generates typed clients and servers.

## Install

```sh
# From crates.io (requires Rust)
cargo install oag-cli

# Or via the install script (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/urmzd/openapi-generator/main/install.sh | sh
```

Windows users can download binaries from the [latest release](https://github.com/urmzd/openapi-generator/releases/latest).

## Commands

### `oag generate`

Generate code from an OpenAPI spec using the configuration in `.urmzd.oag.yaml`.

```sh
# Use config file (loads .urmzd.oag.yaml from current directory)
oag generate

# Override the input spec path
oag generate -i other-spec.yaml
```

**Flags:**

| Flag | Description |
|------|-------------|
| `-i, --input <PATH>` | Path to the OpenAPI spec file (YAML or JSON). Overrides the `input` field in the config file. |

**Behavior:**

1. Loads `.urmzd.oag.yaml` from the current directory (falls back to defaults if missing)
2. Parses the OpenAPI spec and transforms it into an intermediate representation (IR)
3. For each generator in the `generators` map, generates code into the specified output directory
4. Writes a `README.md` in each output directory warning against manual edits
5. Auto-runs formatters if their config files are detected (e.g., `biome.json`, `ruff.toml`)

### `oag validate`

Validate an OpenAPI spec and report its contents without generating any code.

```sh
oag validate -i openapi.yaml
```

**Flags:**

| Flag | Description |
|------|-------------|
| `-i, --input <PATH>` | **(required)** Path to the OpenAPI spec file to validate |

**Output:** Reports the spec version, title, number of paths, schemas, operations, and IR schemas. Exits with a non-zero code if the spec is invalid.

### `oag inspect`

Dump the parsed intermediate representation (IR) so you can see exactly what oag understands from your spec. Useful for debugging or understanding how your spec maps to generated code.

```sh
# YAML output (default)
oag inspect -i openapi.yaml

# JSON output
oag inspect -i openapi.yaml --format json
```

**Flags:**

| Flag | Description |
|------|-------------|
| `-i, --input <PATH>` | **(required)** Path to the OpenAPI spec file |
| `--format <FORMAT>` | Output format: `yaml` (default) or `json` |

**Output:** A structured summary containing:
- `info` — spec title and version
- `schemas` — all resolved schemas with their name and kind (object, enum, alias, union)
- `operations` — all operations with name, method, path, return kind (standard, sse, void), and tags
- `modules` — operation groups (by tag)

### `oag init`

Create a `.urmzd.oag.yaml` configuration file in the current directory with sensible defaults and commented-out examples for all generators.

```sh
# Create config (fails if it already exists)
oag init

# Overwrite an existing config
oag init --force
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--force` | Overwrite an existing `.urmzd.oag.yaml` file |

### `oag completions`

Generate shell completion scripts for tab-completion of commands and flags.

```sh
# Bash
oag completions bash >> ~/.bashrc

# Zsh
oag completions zsh >> ~/.zshrc

# Fish
oag completions fish > ~/.config/fish/completions/oag.fish

# PowerShell
oag completions powershell >> $PROFILE
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<SHELL>` | **(required)** Target shell: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

## Configuration

The CLI automatically loads `.urmzd.oag.yaml` from the current directory. Run `oag init` to create one with defaults.

See the [root README](../../README.md#configuration) for the full configuration reference, including all generator options, layout modes, scaffold settings, and naming strategies.

## Environment variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Control log verbosity (e.g., `RUST_LOG=debug oag generate`). Uses [env_logger](https://docs.rs/env_logger/) syntax. |

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (invalid spec, missing config, I/O failure, generation error) |

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, config, and `CodeGenerator` trait
- [`oag-node-client`](../oag-node-client/) — TypeScript/Node client generator
- [`oag-react-swr-client`](../oag-react-swr-client/) — React/SWR hooks generator
- [`oag-fastapi-server`](../oag-fastapi-server/) — Python FastAPI server generator

## Part of [oag](../../README.md)
