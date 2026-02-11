pub mod config;
pub mod error;
pub mod ir;
pub mod parse;
pub mod transform;

/// A generated file with path and content.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Trait for code generators that produce files from an IR spec.
pub trait CodeGenerator {
    type Config;
    type Error: std::error::Error;
    fn generate(
        &self,
        ir: &ir::IrSpec,
        config: &Self::Config,
    ) -> Result<Vec<GeneratedFile>, Self::Error>;
}
