# oag-react-swr-client

React/SWR hooks generator for OpenAPI 3.x specs.

Extends the Node client generator with React-specific code: SWR hooks for data fetching and a context provider for the API client.

## Layout modes

This generator currently supports **modular** layout only.

In modular mode, the generator produces everything from [`oag-node-client`](../oag-node-client/) plus:

| File | Description |
|------|-------------|
| `src/hooks.tsx` | Typed React hooks for every operation |
| `src/provider.tsx` | `ApiProvider` context component and `useApiClient()` hook |
| `src/index.tsx` | Enhanced barrel exports (includes hooks and provider) |
| `src/client.test.ts` | vitest tests for `ApiClient` (optional, `scaffold.test_runner`) |
| `src/hooks.test.tsx` | vitest smoke tests verifying each hook is exported (optional, `scaffold.test_runner`) |

Source files are placed in a configurable subdirectory (default `src/`) controlled by the `source_dir` generator option. Scaffold files (`package.json`, `tsconfig.json`, etc.) remain at the output root.

## Hook types

| HTTP method | Hook pattern | Library |
|-------------|-------------|---------|
| `GET` | `useSWR` query hook | [SWR](https://swr.vercel.app/) |
| `POST`, `PUT`, `DELETE`, `PATCH` | `useSWRMutation` mutation hook | [SWR](https://swr.vercel.app/) |
| SSE streaming | Custom hook with `useState` + `useCallback` | React |

## Usage pattern

```tsx
import { ApiProvider, useListPets } from "./generated";

function App() {
  return (
    <ApiProvider baseUrl="https://api.example.com">
      <PetList />
    </ApiProvider>
  );
}

function PetList() {
  const { data, error, isLoading } = useListPets();
  // ...
}
```

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, and `CodeGenerator` trait
- [`oag-node-client`](../oag-node-client/) — base TypeScript generation (React generator calls it internally)

## Part of [oag](../../README.md)
