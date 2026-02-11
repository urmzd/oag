/// Emit `main.py` — FastAPI app entry point.
pub fn emit_app() -> String {
    include_str!("../../templates/app.py.j2").to_string()
}
