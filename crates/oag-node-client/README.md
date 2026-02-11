# oag-node-client

TypeScript/Node API client generator for OpenAPI 3.x specs.

Takes an `IrSpec` from `oag-core` and produces a fully typed API client with zero runtime dependencies.

## Layout modes

This generator supports three layout modes:

### bundled
Everything in a single file: `index.ts`

### modular (default)
Separate files per concern:

| File | Description |
|------|-------------|
| `types.ts` | All interfaces, enums, type aliases, and discriminated unions |
| `client.ts` | `ApiClient` class with typed methods for every operation |
| `sse.ts` | SSE streaming utilities (`streamSse` function, `SSEError`, `SSEOptions`) |
| `index.ts` | Barrel exports |

### split
Separate files per operation group (by tag, operation, or route prefix). For example, when splitting by tag:
- `pets.ts` — All operations tagged with "pets"
- `users.ts` — All operations tagged with "users"
- `index.ts` — Barrel exports

When scaffold generation is enabled (default), these are also created:

| File | Description |
|------|-------------|
| `package.json` | npm package with name derived from the spec title |
| `tsconfig.json` | TypeScript compiler configuration |
| `biome.json` | Biome formatter and linter config (optional, `scaffold.formatter`) |
| `tsdown.config.ts` | tsdown bundler config (optional, `scaffold.bundler`) |
| `client.test.ts` | vitest tests for `ApiClient` (optional, `scaffold.test_runner`) |

When `scaffold.test_runner` is enabled (default), `package.json` includes vitest as a dev dependency and a `"test": "vitest run"` script. The generated tests cover:

- Client instantiation (with config, custom headers, custom fetch)
- Per-operation: method existence, correct HTTP method and URL, request body handling, error throwing
- Void operations: returns `undefined` on 204
- SSE operations: returns async iterable

## Key features

- **Zero runtime dependencies** — the generated client uses only `fetch` and standard APIs
- **SSE streaming** — Server-Sent Events are exposed as `AsyncGenerator` functions
- **Request interceptor** — the `ApiClient` accepts an optional interceptor for auth headers, logging, etc.
- **Full type safety** — every parameter, request body, and response is typed
- **JSDoc comments** — generated from spec descriptions (disable with `no_jsdoc: true`)

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, and `CodeGenerator` trait

## Part of [oag](../../README.md)
