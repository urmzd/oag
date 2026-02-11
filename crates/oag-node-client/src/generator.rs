use oag_core::config::{GeneratorConfig, GeneratorId, OutputLayout, SplitBy, ToolSetting};
use oag_core::ir::IrSpec;
use oag_core::{CodeGenerator, GeneratedFile, GeneratorError};

use crate::emitters;
use crate::emitters::scaffold::{NodeScaffoldConfig, ScaffoldOptions};
use crate::emitters::source_path;

/// TypeScript/Node code generator.
pub struct NodeClientGenerator;

impl NodeClientGenerator {
    /// Build scaffold options from a GeneratorConfig.
    pub fn build_scaffold_options(
        ir: &IrSpec,
        config: &GeneratorConfig,
        react: bool,
    ) -> Option<ScaffoldOptions> {
        let raw = config.scaffold.as_ref()?;
        let scaffold: NodeScaffoldConfig = serde_json::from_value(raw.clone()).ok()?;
        Some(ScaffoldOptions {
            name: ir.info.title.clone(),
            package_name: scaffold.package_name,
            repository: scaffold.repository,
            formatter: ToolSetting::resolve(scaffold.formatter.as_ref(), "biome").map(String::from),
            test_runner: ToolSetting::resolve(scaffold.test_runner.as_ref(), "vitest")
                .map(String::from),
            bundler: ToolSetting::resolve(scaffold.bundler.as_ref(), "tsdown").map(String::from),
            react,
            existing_repo: scaffold.existing_repo.unwrap_or(false),
            source_dir: config.source_dir.clone(),
        })
    }
}

impl CodeGenerator for NodeClientGenerator {
    fn id(&self) -> GeneratorId {
        GeneratorId::NodeClient
    }

    fn generate(
        &self,
        ir: &IrSpec,
        config: &GeneratorConfig,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let no_jsdoc = config.no_jsdoc.unwrap_or(false);
        let sd = &config.source_dir;
        let scaffold_options = Self::build_scaffold_options(ir, config, false);

        let mut files = match config.layout {
            OutputLayout::Bundled => {
                let content = emitters::bundled::emit_bundled(ir, no_jsdoc);
                vec![GeneratedFile {
                    path: source_path(sd, "index.ts"),
                    content,
                }]
            }
            OutputLayout::Modular => {
                vec![
                    GeneratedFile {
                        path: source_path(sd, "types.ts"),
                        content: emitters::types::emit_types(ir),
                    },
                    GeneratedFile {
                        path: source_path(sd, "sse.ts"),
                        content: emitters::sse::emit_sse(),
                    },
                    GeneratedFile {
                        path: source_path(sd, "client.ts"),
                        content: emitters::client::emit_client(ir, no_jsdoc),
                    },
                    GeneratedFile {
                        path: source_path(sd, "index.ts"),
                        content: emitters::index::emit_index(),
                    },
                ]
            }
            OutputLayout::Split => {
                let split_by = config.split_by.unwrap_or(SplitBy::Tag);
                emitters::split::emit_split(ir, no_jsdoc, split_by, sd)
            }
        };

        if let Some(ref scaffold) = scaffold_options {
            files.extend(emitters::scaffold::emit_scaffold(scaffold));

            if scaffold.test_runner.is_some() {
                files.push(GeneratedFile {
                    path: source_path(sd, "client.test.ts"),
                    content: emitters::tests::emit_client_tests(ir),
                });
            }
        }

        Ok(files)
    }
}
