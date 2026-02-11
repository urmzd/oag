pub mod config;
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

/// Unified error type for code generators.
#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("template render failed: {0}")]
    Render(String),

    #[error("generation failed: {0}")]
    Other(String),
}

/// Trait for code generators that produce files from an IR spec.
pub trait CodeGenerator {
    fn id(&self) -> config::GeneratorId;
    fn generate(
        &self,
        ir: &ir::IrSpec,
        config: &config::GeneratorConfig,
    ) -> Result<Vec<GeneratedFile>, GeneratorError>;
}
