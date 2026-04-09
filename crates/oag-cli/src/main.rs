use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use clap_complete::Shell;

use oag_core::GeneratedFile;
use oag_core::config::{self, CONFIG_FILE_NAME, LEGACY_CONFIG_FILE, OagConfig};
use oag_core::engine;
use oag_core::engine::pack::TemplatePack;
use oag_core::engine::resolve;
use oag_core::ir::IrSpec;
use oag_core::parse;
use oag_core::transform::{self, TransformOptions};

mod github;

/// Resolve a template pack, with extends support.
///
/// Resolution order:
/// 1. If no `@ref` and a locally installed pack exists → use it
/// 2. Otherwise → fetch from GitHub (cached by `{pack_id}/{ref}`)
fn resolve_pack(specifier: &str) -> Result<TemplatePack> {
    let (pack_id, git_ref) = github::parse_pack_specifier(specifier);
    let is_pinned = specifier.contains('@');

    let pack = if !is_pinned {
        // Check locally installed packs first
        if let Some(disk_path) = resolve::resolve_pack_path(pack_id, None) {
            TemplatePack::from_dir(&disk_path)
                .map_err(|e| anyhow::anyhow!("failed to load pack '{}': {}", pack_id, e))?
        } else {
            fetch_and_cache(pack_id, git_ref)?
        }
    } else {
        fetch_and_cache(pack_id, git_ref)?
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

/// Download a pack from GitHub into a cache directory and load it.
/// Reuses cached copies if they already exist.
fn fetch_and_cache(pack_id: &str, git_ref: &str) -> Result<TemplatePack> {
    let templates_dir = resolve::templates_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine data directory"))?;
    let cache_dir = templates_dir.join(".cache").join(pack_id).join(git_ref);

    if !cache_dir.join("oag.pack.toml").exists() {
        ui::info(&format!("fetching {pack_id}@{git_ref} from GitHub..."));
        github::download_pack(pack_id, git_ref, &cache_dir)?;
        ui::phase_ok("cached", Some(&format!("{pack_id}@{git_ref}")));
    }

    TemplatePack::from_dir(&cache_dir)
        .map_err(|e| anyhow::anyhow!("failed to load pack '{}@{}': {}", pack_id, git_ref, e))
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

        /// Overwrite scaffold files (package.json, tsconfig.json, etc.) even if they already exist.
        /// By default, scaffold files are only written on initial creation to preserve customizations.
        #[arg(long)]
        force_scaffold: bool,
    },

    /// Validate an OpenAPI spec and report its contents (paths, schemas, operations)
    Validate {
        /// Path to the OpenAPI spec file (YAML or JSON)
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Dump the parsed intermediate representation (IR) as JSON for debugging
    Inspect {
        /// Path to the OpenAPI spec file (YAML or JSON)
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Create an oag.yaml config file and download template packs from GitHub
    Init {
        /// Overwrite an existing oag.yaml file
        #[arg(long)]
        force: bool,

        /// Pack IDs to download (e.g., node-client, fastapi-server)
        #[arg(short, long)]
        pack: Vec<String>,
    },

    /// Run validators (lint, typecheck) on generated output directories
    Check,

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

    /// Self-update oag to the latest release
    Update,
    /// Print the current version
    Version,
}

#[derive(Subcommand)]
enum TemplatesAction {
    /// List installed template packs
    List,
    /// Install a template pack from a local directory or download from GitHub
    Install {
        /// Path to a local template pack directory
        source: Option<PathBuf>,
        /// Download a pack by ID from GitHub (e.g., node-client)
        #[arg(long)]
        id: Option<String>,
    },
    /// Remove an installed template pack
    Remove {
        /// Pack ID to remove
        id: String,
    },
    /// Print the template packs directory path
    Path,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            force_scaffold,
        } => cmd_generate(input, force_scaffold),
        Commands::Check => cmd_check(),
        Commands::Validate { input } => cmd_validate(input),
        Commands::Inspect { input } => cmd_inspect(input),
        Commands::Init { force, pack } => cmd_init(force, pack),
        Commands::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            clap_complete::generate(shell, &mut cmd, "oag", &mut std::io::stdout());
            Ok(())
        }
        Commands::Templates { action } => cmd_templates(action),
        Commands::Update => cmd_update(),
        Commands::Version => {
            println!("oag v{}", env!("CARGO_PKG_VERSION"));
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

/// Write scaffold files, skipping any that already exist on disk (unless `force` is true).
/// Returns the number of files that were skipped.
fn write_scaffold_files(base: &Path, files: &[GeneratedFile], force: bool) -> Result<usize> {
    let mut skipped = 0;
    for file in files {
        let path = base.join(&file.path);
        if !force && path.exists() {
            ui::info(&format!("skipped (exists): {}", path.display()));
            skipped += 1;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        fs::write(&path, &file.content)
            .with_context(|| format!("failed to write {}", path.display()))?;
        ui::phase_ok("wrote", Some(&path.display().to_string()));
    }
    Ok(skipped)
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

fn cmd_generate(input: Option<PathBuf>, force_scaffold: bool) -> Result<()> {
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
        let output = engine::generate(&ir, gen_config, &pack).map_err(|e| anyhow::anyhow!(e))?;

        let output_dir = PathBuf::from(&gen_config.output);
        fs::create_dir_all(&output_dir).with_context(|| {
            format!("failed to create output directory {}", output_dir.display())
        })?;

        // Source files are always overwritten
        write_files(&output_dir, &output.source_files)?;

        // Scaffold files are write-once by default (skip if they exist)
        let skipped = write_scaffold_files(&output_dir, &output.scaffold_files, force_scaffold)?;

        // Add README.md
        let readme_path = output_dir.join("README.md");
        fs::write(&readme_path, readme_content())
            .with_context(|| format!("failed to write {}", readme_path.display()))?;
        ui::phase_ok("wrote", Some(&readme_path.display().to_string()));

        // Auto-run formatters
        try_run_formatter(&output_dir, &pack);

        let total = output.source_files.len() + output.scaffold_files.len() + 1;
        let skip_note = if skipped > 0 {
            format!(", {skipped} scaffold skipped")
        } else {
            String::new()
        };
        ui::phase_ok(
            &format!("generated {total} files{skip_note}"),
            Some(&output_dir.display().to_string()),
        );
    }

    ui::info(
        "source files are always regenerated \u{2014} scaffold files are preserved if they exist",
    );
    Ok(())
}

fn cmd_check() -> Result<()> {
    let cfg = try_load_config()?.unwrap_or_default();

    if cfg.generators.is_empty() {
        ui::warn("No generators configured.");
        return Ok(());
    }

    let mut failures = Vec::new();

    for (gen_id, gen_config) in &cfg.generators {
        let pack = resolve_pack(gen_id.as_str())?;
        let output_dir = PathBuf::from(&gen_config.output);

        if !output_dir.exists() {
            ui::warn(&format!(
                "{} does not exist — skipping",
                output_dir.display()
            ));
            continue;
        }

        for (name, val_config) in &pack.manifest.validators {
            if !output_dir.join(&val_config.detect).exists() {
                continue;
            }

            ui::header(&format!("{gen_id}: {name}"));

            if let Some(setup) = &val_config.setup {
                let setup_parts: Vec<&str> = setup.split_whitespace().collect();
                if !setup_parts.is_empty() {
                    match Command::new(setup_parts[0])
                        .args(&setup_parts[1..])
                        .current_dir(&output_dir)
                        .status()
                    {
                        Ok(s) if s.success() => {}
                        Ok(_) => {
                            ui::warn(&format!("setup failed: {setup}"));
                            failures.push(format!("{gen_id}: {name} (setup)"));
                            continue;
                        }
                        Err(e) => {
                            ui::warn(&format!("{} not found: {e}", setup_parts[0]));
                            failures.push(format!("{gen_id}: {name} (setup)"));
                            continue;
                        }
                    }
                }
            }

            let parts: Vec<&str> = val_config.command.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match Command::new(parts[0])
                .args(&parts[1..])
                .current_dir(&output_dir)
                .status()
            {
                Ok(status) if status.success() => {
                    ui::phase_ok(&format!("passed: {}", val_config.command), None);
                }
                Ok(_) => {
                    ui::warn(&format!("failed: {}", val_config.command));
                    failures.push(format!("{gen_id}: {name}"));
                }
                Err(e) => {
                    ui::warn(&format!("{} not found: {e}", parts[0]));
                    failures.push(format!("{gen_id}: {name}"));
                }
            }
        }
    }

    if !failures.is_empty() {
        anyhow::bail!("validation failed for: {}", failures.join(", "));
    }

    ui::phase_ok("all validators passed", None);
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

fn cmd_inspect(input: PathBuf) -> Result<()> {
    let cfg = OagConfig::default();
    let ir = load_spec(&input, &cfg)?;

    let summary = build_inspect_summary(&ir);
    let json = serde_json::to_string_pretty(&summary)?;
    println!("{}", json);

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

fn cmd_init(force: bool, packs: Vec<String>) -> Result<()> {
    let config_path = PathBuf::from(CONFIG_FILE_NAME);

    if config_path.exists() && !force {
        anyhow::bail!(
            "{} already exists. Use --force to overwrite.",
            config_path.display()
        );
    }

    fs::write(&config_path, config::default_config_content())?;
    ui::phase_ok("created", Some(&config_path.display().to_string()));

    // Download requested packs
    for pack_id in &packs {
        install_pack_from_github(pack_id)?;
    }

    Ok(())
}

/// Download a pack from GitHub and install it locally, resolving `extends` dependencies.
fn install_pack_from_github(specifier: &str) -> Result<()> {
    let (pack_id, git_ref) = github::parse_pack_specifier(specifier);
    let templates_dir = resolve::templates_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine data directory"))?;
    let target = templates_dir.join(pack_id);

    github::download_pack(pack_id, git_ref, &target)?;
    ui::phase_ok("installed", Some(&format!("{pack_id}@{git_ref}")));

    // If pack extends another, download the base too
    let pack = TemplatePack::from_dir(&target).map_err(|e| anyhow::anyhow!(e))?;
    if let Some(ref base_id) = pack.manifest.pack.extends {
        let base_target = templates_dir.join(base_id.as_str());
        if !base_target.join("oag.pack.toml").exists() {
            install_pack_from_github(base_id)?;
        }
    }

    Ok(())
}

fn cmd_update() -> Result<()> {
    eprintln!("current version: {}", env!("CARGO_PKG_VERSION"));
    match agentspec_update::self_update("urmzd/oag", env!("CARGO_PKG_VERSION"), "oag")? {
        agentspec_update::UpdateResult::AlreadyUpToDate => eprintln!("already up to date"),
        agentspec_update::UpdateResult::Updated { from, to } => eprintln!("updated: {from} → {to}"),
    }
    Ok(())
}

fn cmd_templates(action: TemplatesAction) -> Result<()> {
    match action {
        TemplatesAction::List => {
            let installed = resolve::list_installed_packs();
            if installed.is_empty() {
                ui::info("No template packs installed.");
            } else {
                ui::header("Installed template packs");
                for (id, path) in &installed {
                    ui::phase_ok(id, Some(&path.display().to_string()));
                }
            }

            let installed_ids: Vec<&str> = installed.iter().map(|(id, _)| id.as_str()).collect();
            ui::header("Available packs (download with `oag templates install --id <pack>`)");
            for &pack_id in github::KNOWN_PACKS {
                let marker = if installed_ids.contains(&pack_id) {
                    " (installed)"
                } else {
                    ""
                };
                ui::phase_ok(pack_id, Some(marker));
            }
            Ok(())
        }
        TemplatesAction::Install { source, id } => {
            if let Some(pack_id) = id {
                install_pack_from_github(&pack_id)?;
                Ok(())
            } else if let Some(source_path) = source {
                let pack_toml = source_path.join("oag.pack.toml");
                if !pack_toml.exists() {
                    anyhow::bail!("no oag.pack.toml found in {}", source_path.display());
                }
                let pack = TemplatePack::from_dir(&source_path).map_err(|e| anyhow::anyhow!(e))?;
                let id = pack.manifest.pack.id.clone();
                let target =
                    resolve::install_pack(&source_path, &id).map_err(|e| anyhow::anyhow!(e))?;
                ui::phase_ok("installed", Some(&format!("{id} → {}", target.display())));
                Ok(())
            } else {
                anyhow::bail!("provide a source path or use --id <pack_id>");
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
