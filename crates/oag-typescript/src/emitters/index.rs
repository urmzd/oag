/// Emit `index.ts` — barrel re-exports.
pub fn emit_index() -> String {
    include_str!("../../templates/index.ts.j2").to_string()
}
