use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use oag_core::GeneratedFile;
use oag_core::config::{self, CONFIG_FILE_NAME, LEGACY_CONFIG_FILE, OagConfig};
use oag_core::engine;
use oag_core::engine::pack::TemplatePack;
use oag_core::engine::resolve;
use oag_core::ir::IrSpec;
use oag_core::parse;
use oag_core::transform::{self, TransformOptions};

// ── Embedded built-in packs ──────────────────────────────────────────

use include_dir::{Dir, include_dir};

static PACKS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../packs");

/// Load an embedded built-in template pack by ID.
fn load_embedded_pack(pack_id: &str) -> Option<TemplatePack> {
    let pack_dir = PACKS_DIR.get_dir(pack_id)?;

    // include_dir stores files with full relative paths, so we need to
    // construct the path relative to the packs root
    let manifest_path = format!("{}/pack.toml", pack_id);
    let manifest_str = pack_dir
        .get_file(&manifest_path)
        .or_else(|| pack_dir.get_file("pack.toml"))?
        .contents_utf8()?;

    let templates_path = format!("{}/templates", pack_id);
    let templates_dir = pack_dir
        .get_dir(&templates_path)
        .or_else(|| pack_dir.get_dir("templates"))?;
    let template_files: Vec<(String, String)> = templates_dir
        .files()
        .filter_map(|f| {
            let name = f.path().file_name()?.to_string_lossy().to_string();
            let content = f.contents_utf8()?.to_string();
            Some((name, content))
        })
        .collect();

    TemplatePack::from_embedded(manifest_str, template_files).ok()
}

/// Resolve a template pack: disk → embedded, with extends support.
fn resolve_pack(pack_id: &str) -> Result<TemplatePack> {
    let pack = if let Some(disk_path) = resolve::resolve_pack_path(pack_id, None) {
        TemplatePack::from_dir(&disk_path)
            .map_err(|e| anyhow::anyhow!("failed to load pack '{}': {}", pack_id, e))?
    } else if let Some(embedded) = load_embedded_pack(pack_id) {
        embedded
    } else {
        anyhow::bail!(
            "template pack '{}' not found. Run `oag templates list` to see available packs.",
            pack_id
        );
    };

    // Handle extends
    if let Some(ref base_id) = pack.manifest.pack.extends {
        let mut base = resolve_pack(base_id)?;
        base.merge_from(&pack);
        Ok(base)
    } else {
        Ok(pack)
    }
}

// ── UI helpers (all output to stderr) ────────────────────────────────

mod ui {
    use crossterm::style::Stylize;
    use std::io::{self, Write};

    pub fn header(cmd: &str) {
        let mut err = io::stderr();
        let _ = writeln!(err);
        let _ = writeln!(err, "  {}", cmd.cyan().bold());
        let _ = writeln!(err, "  {}", "\u{2500}".repeat(40).dim());
        let _ = writeln!(err);
    }

    pub fn phase_ok(msg: &str, detail: Option<&str>) {
        let mut err = io::stderr();
        let suffix = detail
            .map(|d| format!(" \u{00b7} {}", d.dim()))
            .unwrap_or_default();
        let _ = writeln!(err, "  {} {msg}{suffix}", "\u{2713}".green().bold());
    }

    pub fn warn(msg: &str) {
        let mut err = io::stderr();
        let _ = writeln!(err, "  {} {}", "\u{26a0}".yellow().bold(), msg.yellow());
    }

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

    /// Manage template packs for code generation
    Templates {
        #[command(subcommand)]
        action: TemplatesAction,
    },
}

#[derive(Subcommand)]
enum TemplatesAction {
    /// List installed template packs
    List,
    /// Install a template pack from a local directory or extract built-in packs
    Install {
        /// Path to a template pack directory, or --builtin to extract all built-in packs
        source: Option<PathBuf>,
        /// Extract all built-in packs to the templates directory
        #[arg(long)]
        builtin: bool,
    },
    /// Remove an installed template pack
    Remove {
        /// Pack ID to remove
        id: String,
    },
    /// Print the template packs directory path
    Path,
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
        Commands::Templates { action } => cmd_templates(action),
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

/// Try to run formatters based on pack's formatter config or file presence.
fn try_run_formatter(output_dir: &Path, pack: &TemplatePack) {
    for fmt_config in pack.manifest.formatters.values() {
        if output_dir.join(&fmt_config.detect).exists() {
            let parts: Vec<&str> = fmt_config.command.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            match Command::new(parts[0])
                .args(&parts[1..])
                .current_dir(output_dir)
                .output()
            {
                Ok(result) if result.status.success() => {
                    ui::phase_ok(&format!("ran: {}", fmt_config.command), None);
                }
                Ok(_) => {
                    ui::warn(&format!(
                        "{} had issues (non-zero exit)",
                        fmt_config.command
                    ));
                }
                Err(_) => {
                    ui::info(&format!(
                        "{} not found — run `{}` in {} to format",
                        parts[0],
                        fmt_config.command,
                        output_dir.display()
                    ));
                }
            }
        }
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

        let pack = resolve_pack(gen_id.as_str())?;
        let files = engine::generate(&ir, gen_config, &pack).map_err(|e| anyhow::anyhow!(e))?;

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

        // Auto-run formatters
        try_run_formatter(&output_dir, &pack);

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

fn cmd_templates(action: TemplatesAction) -> Result<()> {
    match action {
        TemplatesAction::List => {
            let installed = resolve::list_installed_packs();
            if installed.is_empty() {
                ui::info("No template packs installed on disk.");
            } else {
                ui::header("Installed template packs");
                for (id, path) in &installed {
                    ui::phase_ok(id, Some(&path.display().to_string()));
                }
            }

            // Also list built-in packs
            ui::header("Built-in packs (always available)");
            for dir in PACKS_DIR.dirs() {
                let id = dir.path().file_name().unwrap_or_default().to_string_lossy();
                if let Some(pack_toml) = dir.get_file("pack.toml")
                    && let Some(content) = pack_toml.contents_utf8()
                    && let Ok(manifest) = toml::from_str::<engine::pack::PackManifest>(content)
                {
                    ui::phase_ok(
                        &id,
                        Some(&format!(
                            "{} (v{})",
                            manifest.pack.name, manifest.pack.version
                        )),
                    );
                    continue;
                }
                ui::phase_ok(&id, None);
            }
            Ok(())
        }
        TemplatesAction::Install { source, builtin } => {
            if builtin {
                // Extract all built-in packs
                let Some(templates_dir) = resolve::templates_dir() else {
                    anyhow::bail!("could not determine data directory");
                };
                fs::create_dir_all(&templates_dir)?;

                for dir in PACKS_DIR.dirs() {
                    let id = dir
                        .path()
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let target = templates_dir.join(&id);
                    dir.extract(&target)?;
                    ui::phase_ok("installed", Some(&format!("{id} → {}", target.display())));
                }
                Ok(())
            } else if let Some(source_path) = source {
                // Install from local directory
                let pack_toml = source_path.join("pack.toml");
                if !pack_toml.exists() {
                    anyhow::bail!("no pack.toml found in {}", source_path.display());
                }
                let pack = TemplatePack::from_dir(&source_path).map_err(|e| anyhow::anyhow!(e))?;
                let id = pack.manifest.pack.id.clone();
                let target =
                    resolve::install_pack(&source_path, &id).map_err(|e| anyhow::anyhow!(e))?;
                ui::phase_ok("installed", Some(&format!("{id} → {}", target.display())));
                Ok(())
            } else {
                anyhow::bail!("provide a source path or use --builtin");
            }
        }
        TemplatesAction::Remove { id } => {
            resolve::remove_pack(&id).map_err(|e| anyhow::anyhow!(e))?;
            ui::phase_ok("removed", Some(&id));
            Ok(())
        }
        TemplatesAction::Path => {
            match resolve::templates_dir() {
                Some(dir) => println!("{}", dir.display()),
                None => anyhow::bail!("could not determine data directory"),
            }
            Ok(())
        }
    }
}
