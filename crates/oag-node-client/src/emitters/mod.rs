pub mod bundled;
pub mod client;
pub mod index;
pub mod scaffold;
pub mod split;
pub mod sse;
pub mod tests;
pub mod types;

/// Build a file path under the configured source directory.
///
/// - `source_dir = "src"` → `"src/index.ts"`
/// - `source_dir = ""` → `"index.ts"`
/// - `source_dir = "lib"` → `"lib/index.ts"`
pub fn source_path(source_dir: &str, file: &str) -> String {
    if source_dir.is_empty() {
        file.to_string()
    } else {
        format!("{source_dir}/{file}")
    }
}
