use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use oag_core::config::{self, CONFIG_FILE_NAME, GeneratorId, OagConfig};
use oag_core::ir::IrSpec;
use oag_core::parse;
use oag_core::transform::{self, TransformOptions};
use oag_core::{CodeGenerator, GeneratedFile};
use oag_fastapi_server::FastapiServerGenerator;
use oag_node_client::NodeClientGenerator;
use oag_react_swr_client::ReactSwrClientGenerator;

#[derive(Parser)]
#[command(name = "oag", about = "OpenAPI 3.x code generator", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from an OpenAPI spec
    Generate {
        /// Path to the OpenAPI spec file (YAML or JSON)
        #[arg(short, long)]
        input: Option<PathBuf>,
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
enum InspectFormat {
    Yaml,
    Json,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { input } => cmd_generate(input),

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

/// Look up a generator by its ID.
fn get_generator(id: &GeneratorId) -> Box<dyn CodeGenerator> {
    match id {
        GeneratorId::NodeClient => Box::new(NodeClientGenerator),
        GeneratorId::ReactSwrClient => Box::new(ReactSwrClientGenerator),
        GeneratorId::FastapiServer => Box::new(FastapiServerGenerator),
    }
}

/// Write generated files to disk under the given base directory.
fn write_files(base: &Path, files: &[GeneratedFile]) -> Result<()> {
    for file in files {
        let path = base.join(&file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        fs::write(&path, &file.content)
            .with_context(|| format!("failed to write {}", path.display()))?;
        eprintln!("  wrote {}", path.display());
    }
    Ok(())
}

/// Try to run formatters on the output directory based on config file presence.
fn try_run_formatter(output_dir: &Path) {
    if output_dir.join("biome.json").exists() {
        try_run_biome(output_dir);
    }
    if output_dir.join("ruff.toml").exists() {
        try_run_ruff(output_dir);
    }
}

/// Try to run Biome formatter on the output directory.
fn try_run_biome(output_dir: &Path) {
    match Command::new("npx")
        .args(["@biomejs/biome", "check", "--write", "."])
        .current_dir(output_dir)
        .output()
    {
        Ok(result) if result.status.success() => {
            eprintln!("  formatted with biome");
        }
        Ok(_result) => {
            eprintln!(
                "  warning: biome formatting had issues (non-zero exit), output may need manual formatting"
            );
        }
        Err(_) => {
            eprintln!(
                "  note: biome not found — run `npx @biomejs/biome check --write .` in {} to format",
                output_dir.display()
            );
        }
    }
}

/// Try to run Ruff formatter and linter on the output directory.
fn try_run_ruff(output_dir: &Path) {
    match Command::new("ruff")
        .args(["format", "."])
        .current_dir(output_dir)
        .output()
    {
        Ok(result) if result.status.success() => {
            eprintln!("  formatted with ruff");
        }
        Ok(_) => {
            eprintln!("  warning: ruff format had issues (non-zero exit)");
        }
        Err(_) => {
            eprintln!(
                "  note: ruff not found — run `ruff format . && ruff check --fix .` in {} to format",
                output_dir.display()
            );
            return;
        }
    }

    match Command::new("ruff")
        .args(["check", "--fix", "."])
        .current_dir(output_dir)
        .output()
    {
        Ok(result) if result.status.success() => {
            eprintln!("  linted with ruff");
        }
        Ok(_) => {
            eprintln!("  warning: ruff check had issues (non-zero exit)");
        }
        Err(_) => {}
    }
}

/// Generate the "do not edit" README.
fn readme_content() -> &'static str {
    r#"# Generated Code — Do Not Edit

This directory is **auto-generated** by [oag](https://github.com/urmzd/openapi-generator).
Any manual changes will be overwritten the next time `oag generate` is run.

To regenerate, run:
```
oag generate
```

To customize the generated output, edit your `.urmzd.oag.yaml` configuration file.
"#
}

fn cmd_generate(input: Option<PathBuf>) -> Result<()> {
    let cfg = try_load_config()?.unwrap_or_default();
    let input = input.unwrap_or_else(|| PathBuf::from(&cfg.input));
    let ir = load_spec(&input, &cfg)?;

    if cfg.generators.is_empty() {
        eprintln!("No generators configured. Add a `generators` section to your config.");
        return Ok(());
    }

    for (gen_id, gen_config) in &cfg.generators {
        eprintln!("Generating {} → {}", gen_id, gen_config.output);
        let generator = get_generator(gen_id);
        let files = generator
            .generate(&ir, gen_config)
            .map_err(|e| anyhow::anyhow!(e))?;

        let output_dir = PathBuf::from(&gen_config.output);
        fs::create_dir_all(&output_dir).with_context(|| {
            format!("failed to create output directory {}", output_dir.display())
        })?;

        write_files(&output_dir, &files)?;

        // Add README.md
        let readme_path = output_dir.join("README.md");
        fs::write(&readme_path, readme_content())
            .with_context(|| format!("failed to write {}", readme_path.display()))?;
        eprintln!("  wrote {}", readme_path.display());

        // Auto-run formatter based on config file presence
        try_run_formatter(&output_dir);

        eprintln!(
            "Generated {} files in {}",
            files.len() + 1, // +1 for README
            output_dir.display()
        );
    }

    eprintln!(
        "\nThe generated directories should not be edited manually — changes will be overwritten."
    );
    Ok(())
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
