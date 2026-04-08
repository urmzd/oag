pub mod config;
pub mod engine;
pub mod error;
pub mod ir;
pub mod parse;
pub mod transform;

use thiserror::Error;

/// A generated file with path and content.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Output from `engine::generate()`, separating source files (always overwritten)
/// from scaffold files (write-once by default).
#[derive(Debug, Clone)]
pub struct GenerateOutput {
    /// Source files derived from the OpenAPI spec — always overwritten on each run.
    pub source_files: Vec<GeneratedFile>,
    /// Project scaffold files (package.json, tsconfig, etc.) — only written if they
    /// don't already exist on disk. Use `--force-scaffold` to override.
    pub scaffold_files: Vec<GeneratedFile>,
}

/// Normalize whitespace in generated code:
/// - Collapse 3+ consecutive newlines into 2 (max one blank line)
/// - Ensure trailing newline
pub fn normalize_generated(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut newline_count = 0;
    for ch in content.chars() {
        if ch == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(ch);
            }
        } else {
            newline_count = 0;
            result.push(ch);
        }
    }
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Unified error type for code generators.
#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("template render failed: {0}")]
    Render(String),

    #[error("generation failed: {0}")]
    Other(String),
}
