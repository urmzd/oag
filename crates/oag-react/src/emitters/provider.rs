/// Emit `provider.ts` — React context provider for the API client.
pub fn emit_provider() -> String {
    include_str!("../../templates/provider.ts.j2").to_string()
}
