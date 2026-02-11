# oag-core

OpenAPI 3.x parser, intermediate representation, and transform pipeline.

This is the foundation crate — every other `oag` crate depends on it.

## What it does

- Parses OpenAPI 3.x specs (YAML and JSON)
- Resolves all `$ref` pointers into concrete types
- Transforms specs into a typed intermediate representation (`IrSpec`)
- Normalizes names into PascalCase, camelCase, snake_case, and SCREAMING_SNAKE_CASE
- Detects Server-Sent Events streaming endpoints
- Groups operations into modules by tag

## Transform pipeline

The spec-to-IR transform runs in five phases:

1. **Resolve refs** — inline all `$ref` pointers so the spec is self-contained
2. **Schemas** — convert OpenAPI schema objects into `IrSchema` variants (object, enum, alias, union)
3. **Operations** — convert each path + method into an `IrOperation` with typed parameters, request body, and return type
4. **Modules** — group operations by their first tag into `IrModule`
5. **Info** — extract title, description, version, and server URLs

## Key types

| Type | Description |
|------|-------------|
| `IrSpec` | Top-level IR: info, servers, schemas, operations, modules |
| `IrSchema` | Schema variant: `Object`, `Enum`, `Alias`, `Union` |
| `IrOperation` | A single API operation with method, path, parameters, and return type |
| `IrType` | Primitive and composite types (String, Array, Ref, Union, Map, etc.) |
| `NormalizedName` | A name in all four case conventions |
| `OagConfig` | Parsed `.urmzd.oag.yaml` configuration |
| `GeneratorId` | Enum identifying each generator: `NodeClient`, `ReactSwrClient`, `FastapiServer` |
| `GeneratorConfig` | Per-generator configuration (output, layout, scaffold options, etc.) |
| `CodeGenerator` | Trait that all generators implement |
| `GeneratorError` | Unified error type for generator failures |
| `GeneratedFile` | Output file with path and content |

## `CodeGenerator` trait

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

Each generator implements this trait with:
- **`id()`** — Returns a unique identifier (`GeneratorId::NodeClient`, `GeneratorId::ReactSwrClient`, or `GeneratorId::FastapiServer`)
- **`generate()`** — Transforms the IR into a list of files using the provided configuration

The trait uses a unified `GeneratorConfig` type and `GeneratorError`, simplifying the plugin architecture and allowing the CLI to treat all generators uniformly.

## Part of [oag](../../README.md)
