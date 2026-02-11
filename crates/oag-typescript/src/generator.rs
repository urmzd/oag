use oag_core::ir::IrSpec;
use oag_core::{CodeGenerator, GeneratedFile};
use thiserror::Error;

use crate::emitters;
use crate::emitters::scaffold::ScaffoldOptions;

#[derive(Debug, Error)]
pub enum TypeScriptError {
    #[error("template render failed: {0}")]
    Render(String),
}

/// Configuration for the TypeScript generator.
#[derive(Debug, Clone, Default)]
pub struct TypeScriptConfig {
    pub base_url: Option<String>,
    pub no_jsdoc: bool,
    /// Generate scaffold files (package.json, tsconfig.json, etc).
    pub scaffold: Option<ScaffoldOptions>,
}

/// TypeScript/Node code generator.
pub struct TypeScriptGenerator;

impl CodeGenerator for TypeScriptGenerator {
    type Config = TypeScriptConfig;
    type Error = TypeScriptError;

    fn generate(
        &self,
        ir: &IrSpec,
        config: &Self::Config,
    ) -> Result<Vec<GeneratedFile>, Self::Error> {
        let mut files = vec![
            GeneratedFile {
                path: "types.ts".to_string(),
                content: emitters::types::emit_types(ir),
            },
            GeneratedFile {
                path: "sse.ts".to_string(),
                content: emitters::sse::emit_sse(),
            },
            GeneratedFile {
                path: "client.ts".to_string(),
                content: emitters::client::emit_client(ir, config.no_jsdoc),
            },
            GeneratedFile {
                path: "index.ts".to_string(),
                content: emitters::index::emit_index(),
            },
        ];

        if let Some(ref scaffold) = config.scaffold {
            files.extend(emitters::scaffold::emit_scaffold(scaffold));
        }

        Ok(files)
    }
}
