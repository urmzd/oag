---
name: openapi-generate
description: Generate TypeScript clients, React/SWR hooks, or Python FastAPI servers from OpenAPI 3.x specs using oag. Use when generating API clients, server stubs, or working with OpenAPI code generation.
argument-hint: [spec-path]
---

# OpenAPI Code Generation

Generate code from an OpenAPI 3.x spec using `oag`.

## Steps

1. Ensure a `.urmzd.oag.yaml` config exists. If not, run `oag init` to create one.
2. Configure the generators you need in `.urmzd.oag.yaml`:
   - `node-client` — TypeScript/Node API client
   - `react-swr-client` — React/SWR hooks
   - `fastapi-server` — Python FastAPI server stubs
3. Run `oag generate` (or `oag generate -i $ARGUMENTS` if a spec path is provided).
4. If generation fails, run `oag validate -i <spec>` to check the spec for issues.
5. Use `oag inspect -i <spec>` to dump the parsed IR for debugging.

## Layout Modes

- **bundled** — Single file output
- **modular** — Separate files per concern (types, client, sse, index)
- **split** — Separate files per operation group (by tag, operation, or route)

## Build & Test

```sh
just build    # Build workspace
just test     # Run tests
just lint     # Clippy
just examples # Regenerate all examples
```
