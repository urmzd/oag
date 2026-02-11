use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use oag_core::config::{self, CONFIG_FILE_NAME, OagConfig, OutputLayout, TargetKind};
use oag_core::ir::IrSpec;
use oag_core::parse;
use oag_core::transform::{self, TransformOptions};
use oag_core::{CodeGenerator, GeneratedFile};
use oag_react::{ReactConfig, ReactGenerator};
use oag_typescript::emitters::scaffold::ScaffoldOptions;
use oag_typescript::{TypeScriptConfig, TypeScriptGenerator};

#[derive(Parser)]
#[command(name = "oag", about = "OpenAPI 3.2 code generator", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate client code from an OpenAPI spec
    Generate {
        /// Path to the OpenAPI spec file (YAML or JSON)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output directory for generated code
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target language/framework
        #[arg(short, long)]
        target: Option<Target>,

        /// Base URL override for the API client
        #[arg(long)]
        base_url: Option<String>,

        /// Disable JSDoc comments in generated code
        #[arg(long)]
        no_jsdoc: bool,
    },

    /// Validate an OpenAPI spec
    Validate {
        /// Path to the OpenAPI spec file
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Inspect the parsed IR of an OpenAPI spec
    Inspect {
        /// Path to the OpenAPI spec file
        #[arg(short, long)]
        input: PathBuf,

        /// Output format
        #[arg(long, default_value = "yaml")]
        format: InspectFormat,
    },

    /// Initialize a new oag configuration
    Init {
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Clone, ValueEnum)]
enum Target {
    Typescript,
    React,
    All,
}

impl From<TargetKind> for Target {
    fn from(kind: TargetKind) -> Self {
        match kind {
            TargetKind::Typescript => Target::Typescript,
            TargetKind::React => Target::React,
            TargetKind::All => Target::All,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum InspectFormat {
    Yaml,
    Json,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            output,
            target,
            base_url,
            no_jsdoc,
        } => cmd_generate(input, output, target, base_url, no_jsdoc),

        Commands::Validate { input } => cmd_validate(input),

        Commands::Inspect { input, format } => cmd_inspect(input, format),

        Commands::Init { force } => cmd_init(force),

        Commands::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            clap_complete::generate(shell, &mut cmd, "oag", &mut std::io::stdout());
            Ok(())
        }
    }
}

/// Try to load the project config file from the current directory.
fn try_load_config() -> Result<Option<OagConfig>> {
    let config_path = PathBuf::from(CONFIG_FILE_NAME);
    config::load_config(&config_path).map_err(|e| anyhow::anyhow!(e))
}

fn load_spec(path: &PathBuf, cfg: &OagConfig) -> Result<IrSpec> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    let parsed = match ext {
        "json" => parse::from_json(&content)?,
        _ => parse::from_yaml(&content)?,
    };

    let options = TransformOptions {
        naming_strategy: cfg.naming.strategy,
        aliases: cfg.naming.aliases.clone(),
    };

    let ir = transform::transform_with_options(&parsed, &options)?;
    Ok(ir)
}

fn cmd_generate(
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    target: Option<Target>,
    base_url: Option<String>,
    no_jsdoc: bool,
) -> Result<()> {
    // Load config file if present
    let cfg = try_load_config()?.unwrap_or_default();

    // CLI args override config
    let input = input.unwrap_or_else(|| PathBuf::from(&cfg.input));
    let output = output.unwrap_or_else(|| PathBuf::from(&cfg.output));
    let target = target.unwrap_or_else(|| cfg.target.into());
    let base_url = base_url.or(cfg.client.base_url.clone());
    let no_jsdoc = no_jsdoc || cfg.client.no_jsdoc;

    let ir = load_spec(&input, &cfg)?;

    fs::create_dir_all(&output)
        .with_context(|| format!("failed to create output directory {}", output.display()))?;

    let is_react_target = matches!(target, Target::React | Target::All);
    let scaffold_options = build_scaffold_options(&ir, &cfg, is_react_target);

    match (&target, &cfg.output_options.layout) {
        // Single layout: all files in one directory
        (Target::Typescript, _) | (Target::React, _) | (Target::All, OutputLayout::Single) => {
            let files: Vec<GeneratedFile> = match target {
                Target::Typescript => {
                    let config = TypeScriptConfig {
                        base_url,
                        no_jsdoc,
                        scaffold: scaffold_options,
                    };
                    TypeScriptGenerator
                        .generate(&ir, &config)
                        .map_err(|e| anyhow::anyhow!(e))?
                }
                Target::React => {
                    let config = ReactConfig {
                        base_url,
                        no_jsdoc,
                        scaffold: scaffold_options,
                    };
                    ReactGenerator
                        .generate(&ir, &config)
                        .map_err(|e| anyhow::anyhow!(e))?
                }
                Target::All => {
                    // In single layout, React generator includes TypeScript files
                    let config = ReactConfig {
                        base_url,
                        no_jsdoc,
                        scaffold: scaffold_options,
                    };
                    ReactGenerator
                        .generate(&ir, &config)
                        .map_err(|e| anyhow::anyhow!(e))?
                }
            };

            for file in &files {
                let path = output.join(&file.path);
                fs::write(&path, &file.content)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                eprintln!("  wrote {}", path.display());
            }

            eprintln!("Generated {} files in {}", files.len(), output.display());
        }

        // Split layout: typescript/ and react/ subdirectories
        (Target::All, OutputLayout::Split) => {
            let ts_dir = output.join("typescript");
            let react_dir = output.join("react");
            fs::create_dir_all(&ts_dir)?;
            fs::create_dir_all(&react_dir)?;

            let ts_config = TypeScriptConfig {
                base_url: base_url.clone(),
                no_jsdoc,
                scaffold: scaffold_options.clone(),
            };
            let react_config = ReactConfig {
                base_url,
                no_jsdoc,
                scaffold: scaffold_options,
            };

            let ts_files = TypeScriptGenerator
                .generate(&ir, &ts_config)
                .map_err(|e| anyhow::anyhow!(e))?;
            let react_files = ReactGenerator
                .generate(&ir, &react_config)
                .map_err(|e| anyhow::anyhow!(e))?;

            for file in &ts_files {
                let path = ts_dir.join(&file.path);
                fs::write(&path, &file.content)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                eprintln!("  wrote {}", path.display());
            }

            for file in &react_files {
                let path = react_dir.join(&file.path);
                fs::write(&path, &file.content)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                eprintln!("  wrote {}", path.display());
            }

            eprintln!(
                "Generated {} TypeScript + {} React files",
                ts_files.len(),
                react_files.len()
            );
        }
    }

    Ok(())
}

/// Build scaffold options from config and IR.
fn build_scaffold_options(ir: &IrSpec, cfg: &OagConfig, react: bool) -> Option<ScaffoldOptions> {
    // Always generate scaffold if biome or tsdown is enabled
    if cfg.output_options.biome || cfg.output_options.tsdown {
        Some(ScaffoldOptions {
            name: ir.info.title.clone(),
            package_name: cfg.output_options.package_name.clone(),
            repository: cfg.output_options.repository.clone(),
            biome: cfg.output_options.biome,
            tsdown: cfg.output_options.tsdown,
            react,
        })
    } else {
        None
    }
}

fn cmd_validate(input: PathBuf) -> Result<()> {
    let content = fs::read_to_string(&input)
        .with_context(|| format!("failed to read {}", input.display()))?;

    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    let parsed = match ext {
        "json" => parse::from_json(&content)?,
        _ => parse::from_yaml(&content)?,
    };

    eprintln!(
        "Valid OpenAPI {} spec: {}",
        parsed.openapi, parsed.info.title
    );
    eprintln!("  Version: {}", parsed.info.version);
    eprintln!("  Paths: {}", parsed.paths.len());

    if let Some(ref components) = parsed.components {
        eprintln!("  Schemas: {}", components.schemas.len());
    }

    // Also validate that it transforms to IR successfully
    let ir = transform::transform(&parsed)?;
    eprintln!("  Operations: {}", ir.operations.len());
    eprintln!("  IR Schemas: {}", ir.schemas.len());

    eprintln!("Validation successful.");
    Ok(())
}

fn cmd_inspect(input: PathBuf, format: InspectFormat) -> Result<()> {
    let cfg = OagConfig::default();
    let ir = load_spec(&input, &cfg)?;

    // Build a summary structure
    let summary = build_inspect_summary(&ir);

    match format {
        InspectFormat::Yaml => {
            let yaml = serde_yaml_ng::to_string(&summary)?;
            print!("{}", yaml);
        }
        InspectFormat::Json => {
            let json = serde_json::to_string_pretty(&summary)?;
            println!("{}", json);
        }
    }

    Ok(())
}

fn build_inspect_summary(ir: &IrSpec) -> serde_json::Value {
    let schemas: Vec<serde_json::Value> = ir
        .schemas
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name().pascal_case,
                "kind": match s {
                    oag_core::ir::IrSchema::Object(_) => "object",
                    oag_core::ir::IrSchema::Enum(_) => "enum",
                    oag_core::ir::IrSchema::Alias(_) => "alias",
                    oag_core::ir::IrSchema::Union(_) => "union",
                },
            })
        })
        .collect();

    let operations: Vec<serde_json::Value> = ir
        .operations
        .iter()
        .map(|op| {
            let return_kind = match &op.return_type {
                oag_core::ir::IrReturnType::Standard(_) => "standard",
                oag_core::ir::IrReturnType::Sse(_) => "sse",
                oag_core::ir::IrReturnType::Void => "void",
            };
            serde_json::json!({
                "name": op.name.camel_case,
                "method": op.method.as_str(),
                "path": op.path,
                "return_kind": return_kind,
                "tags": op.tags,
            })
        })
        .collect();

    serde_json::json!({
        "info": {
            "title": ir.info.title,
            "version": ir.info.version,
        },
        "schemas": schemas,
        "operations": operations,
        "modules": ir.modules.iter().map(|m| &m.name.original).collect::<Vec<_>>(),
    })
}

fn cmd_init(force: bool) -> Result<()> {
    let config_path = PathBuf::from(CONFIG_FILE_NAME);

    if config_path.exists() && !force {
        anyhow::bail!(
            "{} already exists. Use --force to overwrite.",
            config_path.display()
        );
    }

    fs::write(&config_path, config::default_config_content())?;
    eprintln!("Created {}", config_path.display());
    Ok(())
}
