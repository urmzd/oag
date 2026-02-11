/// Emit `index.ts` — barrel re-exports for React.
pub fn emit_index() -> String {
    include_str!("../../templates/index.ts.j2").to_string()
}
