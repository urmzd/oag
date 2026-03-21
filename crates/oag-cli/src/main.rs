use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use oag_core::config::{self, CONFIG_FILE_NAME, GeneratorId, LEGACY_CONFIG_FILE, OagConfig};
use oag_core::ir::IrSpec;
use oag_core::parse;
use oag_core::transform::{self, TransformOptions};
use oag_core::{CodeGenerator, GeneratedFile};
use oag_fastapi_server::FastapiServerGenerator;
use oag_node_client::NodeClientGenerator;
use oag_react_swr_client::ReactSwrClientGenerator;

// ── UI helpers (all output to stderr) ────────────────────────────────

mod ui {
    use crossterm::style::Stylize;
    use std::io::{self, Write};

    /// Print a command header: cyan bold title + dim horizontal rule.
    pub fn header(cmd: &str) {
        let mut err = io::stderr();
        let _ = writeln!(err);
        let _ = writeln!(err, "  {}", cmd.cyan().bold());
        let _ = writeln!(err, "  {}", "\u{2500}".repeat(40).dim());
        let _ = writeln!(err);
    }

    /// Print a completed phase with green checkmark.
    pub fn phase_ok(msg: &str, detail: Option<&str>) {
        let mut err = io::stderr();
        let suffix = detail
            .map(|d| format!(" \u{00b7} {}", d.dim()))
            .unwrap_or_default();
        let _ = writeln!(err, "  {} {msg}{suffix}", "\u{2713}".green().bold());
    }

    /// Print a warning message.
    pub fn warn(msg: &str) {
        let mut err = io::stderr();
        let _ = writeln!(err, "  {} {}", "\u{26a0}".yellow().bold(), msg.yellow());
    }

    /// Print an info message.
    pub fn info(msg: &str) {
        let mut err = io::stderr();
        let _ = writeln!(err, "  {} {}", "\u{2139}".cyan(), msg.dim());
    }
}

#[derive(Parser)]
#[command(name = "oag", about = "OpenAPI 3.x code generator", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from an OpenAPI spec using oag.yaml configuration
    Generate {
        /// Path to the OpenAPI spec file (YAML or JSON). Overrides the `input` field in the config.
        #[arg(short, long)]
        input: Option<PathBuf>,
    },

    /// Validate an OpenAPI spec and report its contents (paths, schemas, operations)
    Validate {
        /// Path to the OpenAPI spec file (YAML or JSON)
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Dump the parsed intermediate representation (IR) for debugging
    Inspect {
        /// Path to the OpenAPI spec file (YAML or JSON)
        #[arg(short, long)]
        input: PathBuf,

        /// Output format for the IR dump
        #[arg(long, default_value = "yaml")]
        format: InspectFormat,
    },

    /// Create an oag.yaml config file with defaults and commented examples
    Init {
        /// Overwrite an existing oag.yaml file
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completion scripts for tab-completion (bash, zsh, fish, powershell, elvish)
    Completions {
        /// Target shell for completions
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
    match config::find_config(Path::new(".")) {
        Some((path, is_legacy)) => {
            if is_legacy {
                ui::warn(&format!(
                    "{} is deprecated, rename to {} (legacy support will be removed in a future release)",
                    LEGACY_CONFIG_FILE, CONFIG_FILE_NAME,
                ));
            }
            config::load_config(&path).map_err(|e| anyhow::anyhow!(e))
        }
        None => Ok(None),
    }
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
        ui::phase_ok("wrote", Some(&path.display().to_string()));
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
            ui::phase_ok("formatted with biome", None);
        }
        Ok(_result) => {
            ui::warn(
                "biome formatting had issues (non-zero exit), output may need manual formatting",
            );
        }
        Err(_) => {
            ui::info(&format!(
                "biome not found \u{2014} run `npx @biomejs/biome check --write .` in {} to format",
                output_dir.display()
            ));
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
            ui::phase_ok("formatted with ruff", None);
        }
        Ok(_) => {
            ui::warn("ruff format had issues (non-zero exit)");
        }
        Err(_) => {
            ui::info(&format!(
                "ruff not found \u{2014} run `ruff format . && ruff check --fix .` in {} to format",
                output_dir.display()
            ));
            return;
        }
    }

    match Command::new("ruff")
        .args(["check", "--fix", "."])
        .current_dir(output_dir)
        .output()
    {
        Ok(result) if result.status.success() => {
            ui::phase_ok("linted with ruff", None);
        }
        Ok(_) => {
            ui::warn("ruff check had issues (non-zero exit)");
        }
        Err(_) => {}
    }
}

/// Generate the "do not edit" README.
fn readme_content() -> &'static str {
    r#"# Generated Code — Do Not Edit

This directory is **auto-generated** by [oag](https://github.com/urmzd/oag).
Any manual changes will be overwritten the next time `oag generate` is run.

To regenerate, run:
```
oag generate
```

To customize the generated output, edit your `oag.yaml` configuration file.
"#
}

fn cmd_generate(input: Option<PathBuf>) -> Result<()> {
    let cfg = try_load_config()?.unwrap_or_default();
    let input = input.unwrap_or_else(|| PathBuf::from(&cfg.input));
    let ir = load_spec(&input, &cfg)?;

    if cfg.generators.is_empty() {
        ui::warn("No generators configured. Add a `generators` section to your config.");
        return Ok(());
    }

    for (gen_id, gen_config) in &cfg.generators {
        ui::header(&format!(
            "Generating {} \u{2192} {}",
            gen_id, gen_config.output
        ));
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
        ui::phase_ok("wrote", Some(&readme_path.display().to_string()));

        // Auto-run formatter based on config file presence
        try_run_formatter(&output_dir);

        ui::phase_ok(
            &format!("generated {} files", files.len() + 1),
            Some(&output_dir.display().to_string()),
        );
    }

    ui::info(
        "generated directories should not be edited manually \u{2014} changes will be overwritten",
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

    ui::header(&format!("Validate \u{00b7} {}", parsed.info.title));

    ui::info(&format!("OpenAPI {}", parsed.openapi));
    ui::info(&format!("Version: {}", parsed.info.version));
    ui::info(&format!("Paths: {}", parsed.paths.len()));

    if let Some(ref components) = parsed.components {
        ui::info(&format!("Schemas: {}", components.schemas.len()));
    }

    // Also validate that it transforms to IR successfully
    let ir = transform::transform(&parsed)?;
    ui::info(&format!("Operations: {}", ir.operations.len()));
    ui::info(&format!("IR Schemas: {}", ir.schemas.len()));

    ui::phase_ok("validation successful", None);
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
    ui::phase_ok("created", Some(&config_path.display().to_string()));
    Ok(())
}
