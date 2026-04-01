# oag-core

OpenAPI 3.x parser, intermediate representation, transform pipeline, and template pack engine.

This is the foundation crate — `oag-cli` depends on it.

## What it does

- Parses OpenAPI 3.x specs (YAML and JSON)
- Resolves all `$ref` pointers into concrete types
- Transforms specs into a typed intermediate representation (`IrSpec`)
- Normalizes names into PascalCase, camelCase, snake_case, and SCREAMING_SNAKE_CASE
- Detects Server-Sent Events streaming endpoints
- Groups operations into modules by tag
- **Template pack engine** — renders Jinja2 template packs against the IR to generate code

## Transform pipeline

The spec-to-IR transform runs in six phases:

1. **Resolve refs** — inline all `$ref` pointers so the spec is self-contained
2. **Schemas** — convert OpenAPI schema objects into `IrSchema` variants (object, enum, alias, union)
3. **Operations** — convert each path + method into an `IrOperation` with typed parameters, request body, and return type
4. **Modules** — group operations by their first tag into `IrModule`
5. **Info** — extract title, description, version, and server URLs
6. **Promote inline objects** — lift anonymous inline object schemas to named top-level schemas for stronger type safety

## Template pack engine

The engine (`src/engine/`) renders declarative template packs to generate code. Each pack is a directory with a `oag.pack.toml` manifest and Jinja2 templates.

### Engine modules

| Module | Description |
|--------|-------------|
| `engine/mod.rs` | Main `generate()` entry point — orchestrates rendering for all layout modes |
| `engine/pack.rs` | `TemplatePack` and `PackManifest` types — pack loading and structure |
| `engine/context.rs` | Builds the template context from the IR and generator config |
| `engine/resolve.rs` | Pack resolution — finds packs on disk or from embedded sources |
| `engine/type_map.rs` | Declarative type mapping from IR types to target language types |
| `engine/bundled.rs` | Bundled layout rendering (single-file output) |

### Engine API

```rust
pub fn generate(
    ir: &IrSpec,
    config: &GeneratorConfig,
    pack: &TemplatePack,
) -> Result<Vec<GeneratedFile>, GeneratorError>
```

The engine supports three layout modes:
- **modular** — separate files per concern
- **bundled** — single concatenated file
- **split** — separate files per operation group

Packs support **inheritance** via the `extends` field in `oag.pack.toml`, allowing packs like `react-swr-client` to inherit all templates from `node-client` and add its own.

## Key types

| Type | Description |
|------|-------------|
| `IrSpec` | Top-level IR: info, servers, schemas, operations, modules |
| `IrSchema` | Schema variant: `Object`, `Enum`, `Alias`, `Union` |
| `IrOperation` | A single API operation with method, path, parameters, and return type |
| `IrType` | Primitive and composite types (String, Array, Ref, Union, Map, etc.) |
| `NormalizedName` | A name in all four case conventions |
| `OagConfig` | Parsed `oag.yaml` configuration |
| `GeneratorId` | String-based identifier for template packs (e.g., `"node-client"`) |
| `GeneratorConfig` | Per-generator configuration (output, layout, scaffold options, etc.) |
| `TemplatePack` | A loaded template pack with manifest and templates |
| `GeneratorError` | Unified error type for generator failures |
| `GeneratedFile` | Output file with path and content |

## Part of [oag](../../README.md)
