# oag-fastapi-server

Python FastAPI server generator for OpenAPI 3.x specs.

Takes an `IrSpec` from `oag-core` and produces FastAPI route stubs with Pydantic v2 models.

## Layout modes

This generator supports three layout modes:

### bundled
Everything in a single file: `main.py`

### modular (default)
Separate files per concern:

| File | Description |
|------|-------------|
| `models.py` | Pydantic v2 models for all schemas (request/response bodies) |
| `routes.py` | FastAPI route stubs with proper type annotations |
| `sse.py` | Server-Sent Events utilities using `StreamingResponse` |
| `main.py` | FastAPI app entry point |

### split
Separate files per operation group (by tag, operation, or route prefix). For example, when splitting by tag:
- `pets.py` — All operations tagged with "pets"
- `users.py` — All operations tagged with "users"
- `main.py` — FastAPI app that imports all routes

When scaffold generation is enabled (default), these are also created:

| File | Description |
|------|-------------|
| `pyproject.toml` | uv-compatible project config with FastAPI and uvicorn dependencies |
| `conftest.py` | pytest fixture with async `httpx` test client (optional, `scaffold.tests`) |
| `test_routes.py` | Per-operation pytest tests (optional, `scaffold.tests`) |

When `scaffold.tests` is enabled (default), `pyproject.toml` includes a `[dependency-groups]` section (PEP 735) with pytest, pytest-asyncio, and httpx as dev dependencies. The generated tests cover:

- Route existence (not 404)
- Stub returns 500 (NotImplementedError)
- Input validation returns 422 (for operations with request body)
- Unknown path returns 404

## Key features

- **Pydantic v2 models** — All OpenAPI schemas are converted to Pydantic models with proper field types, descriptions, and validation
- **Type-safe routes** — Every FastAPI route is fully annotated with request/response types
- **SSE streaming** — Server-Sent Events endpoints use `StreamingResponse` with async generators (no external dependencies)
- **uv-compatible** — `pyproject.toml` uses PEP 735 dependency groups; run with `uv sync && uv run pytest`
- **Absolute imports** — Generated code uses absolute imports (`from models import ...`) so tests work without package installation
- **Stub implementation** — Routes raise `NotImplementedError`; you fill in the business logic

## Generated route structure

For a `GET /pets` operation, the generator produces:

```python
@router.get("/pets", response_model=list[Pet])
async def list_pets(
    limit: int | None = Query(default=None, description="How many items to return"),
) -> list[Pet]:
    """
    List all pets
    """
    # TODO: Implement this endpoint
    raise HTTPException(status_code=501, detail="Not implemented")
```

For SSE endpoints (detected by `text/event-stream` content type), the generator produces:

```python
@router.get("/chat/stream", response_class=StreamingResponse)
async def stream_chat(
    message: str = Query(..., description="The message to send"),
) -> StreamingResponse:
    """
    Stream chat responses via SSE
    """
    async def event_generator():
        # TODO: Implement your SSE logic here
        yield "data: Example event\n\n"

    return StreamingResponse(event_generator(), media_type="text/event-stream")
```

## Depends on

- [`oag-core`](../oag-core/) — parser, IR, and `CodeGenerator` trait

## Part of [oag](../../README.md)
