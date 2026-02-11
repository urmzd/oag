/// Emit `sse.ts` — the inlined SSE runtime (no external dependencies).
pub fn emit_sse() -> String {
    include_str!("../../templates/sse.ts.j2").to_string()
}
