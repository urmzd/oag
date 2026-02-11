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
| `CodeGenerator` | Trait that all generators implement |
| `GeneratedFile` | Output file with path and content |

## `CodeGenerator` trait

```rust
pub trait CodeGenerator {
    type Config;
    type Error: std::error::Error;
    fn generate(
        &self,
        ir: &IrSpec,
        config: &Self::Config,
    ) -> Result<Vec<GeneratedFile>, Self::Error>;
}
```

Implement this trait to add a new language or framework target.

## Part of [oag](../../README.md)
