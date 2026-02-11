/// Emit `sse.py` — SSE helper using FastAPI's built-in StreamingResponse.
pub fn emit_sse() -> String {
    include_str!("../../templates/sse.py.j2").to_string()
}
