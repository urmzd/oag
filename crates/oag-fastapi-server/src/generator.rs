use oag_core::config::{GeneratorConfig, GeneratorId, ToolSetting};
use oag_core::ir::IrSpec;
use oag_core::{CodeGenerator, GeneratedFile, GeneratorError};

use crate::emitters;
use crate::emitters::scaffold::FastapiScaffoldConfig;

/// FastAPI server stub generator.
pub struct FastapiServerGenerator;

impl CodeGenerator for FastapiServerGenerator {
    fn id(&self) -> GeneratorId {
        GeneratorId::FastapiServer
    }

    fn generate(
        &self,
        ir: &IrSpec,
        config: &GeneratorConfig,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut files = vec![
            GeneratedFile {
                path: "models.py".to_string(),
                content: emitters::models::emit_models(ir),
            },
            GeneratedFile {
                path: "routes.py".to_string(),
                content: emitters::routes::emit_routes(ir),
            },
            GeneratedFile {
                path: "sse.py".to_string(),
                content: emitters::sse::emit_sse(),
            },
            GeneratedFile {
                path: "main.py".to_string(),
                content: emitters::app::emit_app(),
            },
            GeneratedFile {
                path: "__init__.py".to_string(),
                content: String::new(),
            },
        ];

        // Add scaffold (pyproject.toml, optionally ruff.toml)
        if let Some(ref raw) = config.scaffold {
            let scaffold: FastapiScaffoldConfig = serde_json::from_value(raw.clone())
                .map_err(|e| GeneratorError::Other(format!("invalid scaffold config: {e}")))?;
            files.extend(emitters::scaffold::emit_scaffold(&scaffold));

            if ToolSetting::resolve(scaffold.test_runner.as_ref(), "pytest").is_some() {
                files.extend(emitters::tests::emit_tests(ir));
            }
        }

        Ok(files)
    }
}
