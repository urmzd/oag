# oag-typescript

TypeScript client code generator for OpenAPI 3.x specs.

Takes an `IrSpec` from `oag-core` and produces a fully typed API client with zero runtime dependencies.

## Generated files

| File | Description |
|------|-------------|
| `types.ts` | All interfaces, enums, type aliases, and discriminated unions |
| `client.ts` | `ApiClient` class with typed methods for every operation |
| `sse.ts` | SSE streaming utilities (`streamSse` function, `SSEError`, `SSEOptions`) |
| `index.ts` | Barrel exports |

When scaffold generation is enabled (default), these are also created:

| File | Description |
|------|-------------|
| `package.json` | npm package with name derived from the spec title |
| `tsconfig.json` | TypeScript compiler configuration |
| `biome.json` | Biome formatter and linter config |
| `tsdown.config.ts` | tsdown bundler config |

## Key features

- **Zero runtime dependencies** — the generated client uses only `fetch` and standard APIs
- **SSE streaming** — Server-Sent Events are exposed as `AsyncGenerator` functions
- **Request interceptor** — the `ApiClient` accepts an optional interceptor for auth headers, logging, etc.
- **Full type safety** — every parameter, request body, and response is typed
- **JSDoc comments** — generated from spec descriptions (disable with `no_jsdoc: true`)

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, and `CodeGenerator` trait

## Part of [oag](../../README.md)
