# oag-react-swr-client

React/SWR hooks generator for OpenAPI 3.x specs.

Extends the Node client generator with React-specific code: SWR hooks for data fetching and a context provider for the API client.

## Layout modes

This generator currently supports **modular** layout only.

In modular mode, the generator produces everything from [`oag-node-client`](../oag-node-client/) plus:

| File | Description |
|------|-------------|
| `hooks.ts` | Typed React hooks for every operation |
| `provider.ts` | `ApiProvider` context component and `useApiClient()` hook |
| `index.ts` | Enhanced barrel exports (includes hooks and provider) |
| `client.test.ts` | vitest tests for `ApiClient` (optional, `scaffold.tests`) |
| `hooks.test.ts` | vitest smoke tests verifying each hook is exported (optional, `scaffold.tests`) |

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
