use oag_core::ir::IrSpec;
use oag_core::{CodeGenerator, GeneratedFile};
use oag_typescript::emitters::scaffold::ScaffoldOptions;
use oag_typescript::{TypeScriptConfig, TypeScriptGenerator};
use thiserror::Error;

use crate::emitters;

#[derive(Debug, Error)]
pub enum ReactError {
    #[error("typescript generation failed: {0}")]
    TypeScript(#[from] oag_typescript::generator::TypeScriptError),

    #[error("template render failed: {0}")]
    Render(String),
}

/// Configuration for the React generator.
#[derive(Debug, Clone, Default)]
pub struct ReactConfig {
    pub base_url: Option<String>,
    pub no_jsdoc: bool,
    /// Generate scaffold files (package.json, tsconfig.json, etc).
    pub scaffold: Option<ScaffoldOptions>,
}

/// React/SWR code generator. Produces the TypeScript client files plus React hooks.
pub struct ReactGenerator;

impl CodeGenerator for ReactGenerator {
    type Config = ReactConfig;
    type Error = ReactError;

    fn generate(
        &self,
        ir: &IrSpec,
        config: &Self::Config,
    ) -> Result<Vec<GeneratedFile>, Self::Error> {
        // First generate the base TypeScript client files
        let ts_config = TypeScriptConfig {
            base_url: config.base_url.clone(),
            no_jsdoc: config.no_jsdoc,
            scaffold: config.scaffold.clone(),
        };
        let mut files = TypeScriptGenerator.generate(ir, &ts_config)?;

        // Add React-specific files
        files.push(GeneratedFile {
            path: "hooks.ts".to_string(),
            content: emitters::hooks::emit_hooks(ir),
        });

        files.push(GeneratedFile {
            path: "provider.ts".to_string(),
            content: emitters::provider::emit_provider(),
        });

        // Replace index.ts with React version that includes hooks + provider
        if let Some(idx) = files.iter().position(|f| f.path == "index.ts") {
            files[idx] = GeneratedFile {
                path: "index.ts".to_string(),
                content: emitters::index::emit_index(),
            };
        }

        Ok(files)
    }
}
