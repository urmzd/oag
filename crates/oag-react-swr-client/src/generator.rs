use oag_core::config::{GeneratorConfig, GeneratorId};
use oag_core::ir::IrSpec;
use oag_core::{CodeGenerator, GeneratedFile, GeneratorError};
use oag_node_client::NodeClientGenerator;
use oag_node_client::emitters::source_path;

use crate::emitters;

/// React/SWR code generator. Produces the TypeScript client files plus React hooks.
pub struct ReactSwrClientGenerator;

impl CodeGenerator for ReactSwrClientGenerator {
    fn id(&self) -> GeneratorId {
        GeneratorId::ReactSwrClient
    }

    fn generate(
        &self,
        ir: &IrSpec,
        config: &GeneratorConfig,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let scaffold_options = NodeClientGenerator::build_scaffold_options(ir, config, true);

        // Generate base TypeScript client files via the node-client generator
        // We manually produce the files to inject react scaffold options
        let no_jsdoc = config.no_jsdoc.unwrap_or(false);
        let sd = &config.source_dir;
        let mut files = vec![
            GeneratedFile {
                path: source_path(sd, "types.ts"),
                content: oag_node_client::emitters::types::emit_types(ir),
            },
            GeneratedFile {
                path: source_path(sd, "sse.ts"),
                content: oag_node_client::emitters::sse::emit_sse(),
            },
            GeneratedFile {
                path: source_path(sd, "client.ts"),
                content: oag_node_client::emitters::client::emit_client(ir, no_jsdoc),
            },
        ];

        if let Some(ref scaffold) = scaffold_options {
            files.extend(oag_node_client::emitters::scaffold::emit_scaffold(scaffold));

            if scaffold.test_runner.is_some() {
                files.push(GeneratedFile {
                    path: source_path(sd, "client.test.ts"),
                    content: oag_node_client::emitters::tests::emit_client_tests(ir),
                });
                files.push(GeneratedFile {
                    path: source_path(sd, "hooks.test.tsx"),
                    content: emitters::tests::emit_hooks_tests(ir),
                });
            }
        }

        // Add React-specific files
        files.push(GeneratedFile {
            path: source_path(sd, "hooks.tsx"),
            content: emitters::hooks::emit_hooks(ir),
        });

        files.push(GeneratedFile {
            path: source_path(sd, "provider.tsx"),
            content: emitters::provider::emit_provider(),
        });

        // Add React index.tsx (includes hooks + provider exports)
        files.push(GeneratedFile {
            path: source_path(sd, "index.tsx"),
            content: emitters::index::emit_index(),
        });

        Ok(files)
    }
}
