mod self_update;

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
mod packs;

use packs::PackSource;

/// A pack resolved for generation, with where it came from.
struct ResolvedPack {
    pack: TemplatePack,
    /// `(pack id, source, sha256)` for the pack and every pack it extends.
    provenance: Vec<(String, PackSource, String)>,
}

/// Options for pack resolution.
#[derive(Debug, Clone, Copy, Default)]
struct ResolveOptions {
    /// Use a local copy of a built-in pack even when another oag version
    /// installed it (see `packs::stale_reason`).
    allow_stale: bool,
}

/// Resolve a pack, with extends support.
///
/// Resolution never touches the network unless the specifier pins a ref:
/// 1. `id@ref` → that ref from GitHub (explicit and pinned; cached in
///    `.oag/packs/<id>/` by ref and digest). Its `extends` base is pinned to
///    the same ref.
/// 2. a pack directory in `.oag/packs/<id>/` (installed or committed). A copy
///    of a built-in pack installed by another oag version is refused unless
///    `allow_stale` is set.
/// 3. the pack built into this oag binary
fn resolve_pack(specifier: &str, opts: ResolveOptions) -> Result<ResolvedPack> {
    let (pack_id, git_ref) = github::parse_pack_specifier(specifier);

    let (pack, source, sha256) = if let Some(git_ref) = git_ref {
        let (pack, sha256) = fetch_and_install(pack_id, git_ref, false)?;
        (pack, PackSource::GitHub(git_ref.to_string()), sha256)
    } else if let Some(disk_path) = resolve::resolve_pack_path(pack_id, None) {
        if let Some(reason) = packs::stale_reason(pack_id, &disk_path) {
            if !opts.allow_stale {
                anyhow::bail!(
                    "{dir} {reason}, and it overrides the {pack_id} pack built into oag v{v}.\n\
                     Templates from another oag version can generate code that does not compile \
                     against this one, or keep old runtime behaviour.\n\
                     Fix: delete {dir} to use the built-in pack, or run \
                     `oag packs install --id {pack_id}` to replace it with this version's copy \
                     (reapply your edits if you customized it).\n\
                     To keep it deliberately, pin it (`{pack_id}@<ref>` in oag.yaml) or pass \
                     --allow-stale-packs.",
                    dir = disk_path.display(),
                    v = env!("CARGO_PKG_VERSION"),
                );
            }
            ui::warn(&format!(
                "{} {reason}; using it because of --allow-stale-packs",
                disk_path.display()
            ));
        }
        let pack = TemplatePack::from_dir(&disk_path)
            .map_err(|e| anyhow::anyhow!("failed to load pack '{}': {}", pack_id, e))?;
        (pack, PackSource::Local, packs::dir_digest(&disk_path)?)
    } else if let Some(pack) = packs::load_embedded(pack_id) {
        let sha256 = packs::embedded_digest(pack_id).unwrap_or_default();
        (pack?, PackSource::Embedded, sha256)
    } else {
        anyhow::bail!(
            "pack '{pack_id}' is not installed in {} and is not built into oag v{}.\n\
             Install it explicitly: `oag packs install <path>`, or pin a release with \
             `oag packs install --id {pack_id}@v{}`.",
            resolve::templates_dir().display(),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION"),
        );
    };

    let mut provenance = vec![(pack_id.to_string(), source, sha256)];

    // Handle extends: a pinned pack's base is pinned to the same ref, so the
    // pair always comes from one revision.
    if let Some(ref base_id) = pack.manifest.pack.extends {
        let base_spec = match git_ref {
            Some(r) => format!("{base_id}@{r}"),
            None => base_id.clone(),
        };
        let mut base = resolve_pack(&base_spec, opts)?;
        base.pack.merge_from(&pack);
        provenance.append(&mut base.provenance);
        Ok(ResolvedPack {
            pack: base.pack,
            provenance,
        })
    } else {
        Ok(ResolvedPack { pack, provenance })
    }
}

/// Install a pack from GitHub at an explicit ref into `.oag/packs/<id>/`,
/// recording the source and digest. Unless `force` is set, a copy already
/// downloaded at the same ref whose digest still matches its record is reused
/// without touching the network.
fn fetch_and_install(pack_id: &str, git_ref: &str, force: bool) -> Result<(TemplatePack, String)> {
    let target = resolve::templates_dir().join(pack_id);

    let cached = (!force)
        .then(|| packs::read_source_record(&target))
        .flatten()
        .filter(|r| r.source.as_deref() == Some("github") && r.git_ref.as_deref() == Some(git_ref))
        .and_then(|r| r.sha256)
        .filter(|sha| packs::dir_digest(&target).ok().as_ref() == Some(sha));

    let sha256 = match cached {
        Some(sha256) => sha256,
        None => {
            ui::info(&format!("installing {pack_id}@{git_ref} from GitHub..."));
            github::download_pack(pack_id, git_ref, &target)?;
            let sha256 = packs::dir_digest(&target)?;
            packs::write_source_record(&target, &PackSource::GitHub(git_ref.to_string()), &sha256)?;
            ui::phase_ok(
                "installed",
                Some(&format!("{pack_id}@{git_ref} · sha256:{}", short(&sha256))),
            );
            sha256
        }
    };

    let pack = TemplatePack::from_dir(&target)
        .map_err(|e| anyhow::anyhow!("failed to load pack '{}': {}", pack_id, e))?;
    Ok((pack, sha256))
}

/// First 12 hex characters of a digest, for display.
fn short(sha256: &str) -> &str {
    &sha256[..sha256.len().min(12)]
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
        input: Option<PathBuf>,

        /// Overwrite scaffold files (package.json, tsconfig.json, etc.) even if they already exist.
        /// By default, scaffold files are only written on initial creation to preserve customizations.
        #[arg(long)]
        force_scaffold: bool,

        /// Use a copy of a built-in pack in .oag/packs/ even when it was installed by
        /// another oag version (or has no install record).
        #[arg(long)]
        allow_stale_packs: bool,
    },

    /// Validate an OpenAPI spec and report its contents (paths, schemas, operations)
    Validate {
        /// Path to the OpenAPI spec file (YAML or JSON)
        input: PathBuf,
    },

    /// Dump the parsed intermediate representation (IR) as JSON for debugging
    Inspect {
        /// Path to the OpenAPI spec file (YAML or JSON)
        input: PathBuf,
    },

    /// Create an oag.yaml config file and optionally install packs into .oag/packs/
    Init {
        /// Overwrite an existing oag.yaml file
        #[arg(long)]
        force: bool,

        /// Pack IDs to install (e.g., node-client); built-in packs need no network, `id@ref` downloads that ref
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

    /// Manage packs for code generation
    Packs {
        #[command(subcommand)]
        action: PacksAction,
    },

    /// Self-update oag to the latest release
    Update,
    /// Print the current version
    Version,
}

#[derive(Subcommand)]
enum PacksAction {
    /// List installed and built-in packs with their digests
    List,
    /// Install a pack into .oag/packs/ from a local directory, the packs built into oag, or a pinned GitHub ref
    Install {
        /// Path to a local pack directory
        source: Option<PathBuf>,
        /// Install a pack by ID: the built-in copy (e.g. node-client), or `id@ref` to download that ref from GitHub
        #[arg(long)]
        id: Option<String>,
    },
    /// Remove an installed pack
    Remove {
        /// Pack ID to remove
        id: String,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            force_scaffold,
            allow_stale_packs,
        } => cmd_generate(
            input,
            force_scaffold,
            ResolveOptions {
                allow_stale: allow_stale_packs,
            },
        ),
        Commands::Check => cmd_check(),
        Commands::Validate { input } => cmd_validate(input),
        Commands::Inspect { input } => cmd_inspect(input),
        Commands::Init { force, pack } => cmd_init(force, pack),
        Commands::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            clap_complete::generate(shell, &mut cmd, "oag", &mut std::io::stdout());
            Ok(())
        }
        Commands::Packs { action } => cmd_packs(action),
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

/// Generate the "do not edit" README, recording the packs that produced it.
///
/// The pack digests make the output reproducible and checkable: the same oag
/// version and packs give the same digests, and a pack change shows up here.
fn readme_content(provenance: &[(String, PackSource, String)]) -> String {
    let packs: String = provenance
        .iter()
        .map(|(id, _, sha256)| format!("- `{id}` sha256:{sha256}\n"))
        .collect();
    format!(
        r#"# Generated Code — Do Not Edit

This directory is **auto-generated** by [oag](https://github.com/urmzd/oag).
Any manual changes will be overwritten the next time `oag generate` is run.

To regenerate, run:
```
oag generate
```

To customize the generated output, edit your `oag.yaml` configuration file.

Generated with these template packs:

{packs}"#
    )
}

fn cmd_generate(input: Option<PathBuf>, force_scaffold: bool, opts: ResolveOptions) -> Result<()> {
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

        let resolved = resolve_pack(gen_id.as_str(), opts)?;
        for (id, source, sha256) in &resolved.provenance {
            ui::info(&format!("pack {id} · {source} · sha256:{}", short(sha256)));
        }
        let pack = &resolved.pack;
        let output = engine::generate(&ir, gen_config, pack).map_err(|e| anyhow::anyhow!(e))?;

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
        fs::write(&readme_path, readme_content(&resolved.provenance))
            .with_context(|| format!("failed to write {}", readme_path.display()))?;
        ui::phase_ok("wrote", Some(&readme_path.display().to_string()));

        // Auto-run formatters
        try_run_formatter(&output_dir, pack);

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
        // Validators only read the manifest, so a stale local pack is fine here.
        let pack = resolve_pack(gen_id.as_str(), ResolveOptions { allow_stale: true })?.pack;
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

    // Install requested packs
    for pack_id in &packs {
        install_pack_by_id(pack_id)?;
    }

    Ok(())
}

/// Install a pack by ID into `.oag/packs/`, resolving `extends` dependencies.
///
/// Without a ref, the pack built into this binary is installed (no network).
/// `id@ref` downloads that ref from GitHub; a pack that is not built in is
/// downloaded at the tag of this oag version, never from a moving branch.
fn install_pack_by_id(specifier: &str) -> Result<()> {
    let (pack_id, git_ref) = github::parse_pack_specifier(specifier);
    let templates_dir = resolve::templates_dir();
    let target = templates_dir.join(pack_id);

    if git_ref.is_none() && packs::write_embedded(pack_id, &target)? {
        let sha256 = packs::dir_digest(&target)?;
        packs::write_source_record(&target, &PackSource::Embedded, &sha256)?;
        ui::phase_ok(
            "installed",
            Some(&format!(
                "{pack_id} (built into oag v{}) · sha256:{}",
                env!("CARGO_PKG_VERSION"),
                short(&sha256)
            )),
        );
    } else {
        let version_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
        fetch_and_install(pack_id, git_ref.unwrap_or(&version_tag), true)?;
    }

    // If pack extends another, install the base too
    let pack = TemplatePack::from_dir(&target).map_err(|e| anyhow::anyhow!(e))?;
    if let Some(ref base_id) = pack.manifest.pack.extends {
        let base_target = templates_dir.join(base_id.as_str());
        match git_ref {
            // A pinned pack's base comes from the same ref.
            Some(r) => install_pack_by_id(&format!("{base_id}@{r}"))?,
            None if !base_target.join("oag.pack.toml").exists() => install_pack_by_id(base_id)?,
            None => {}
        }
    }

    Ok(())
}

fn cmd_update() -> Result<()> {
    eprintln!("current version: {}", env!("CARGO_PKG_VERSION"));
    match self_update::self_update("urmzd/oag", env!("CARGO_PKG_VERSION"), "oag")? {
        self_update::UpdateResult::AlreadyUpToDate => eprintln!("already up to date"),
        self_update::UpdateResult::Updated { from, to } => eprintln!("updated: {from} → {to}"),
    }
    Ok(())
}

fn cmd_packs(action: PacksAction) -> Result<()> {
    match action {
        PacksAction::List => {
            let installed = resolve::list_installed_packs();
            if installed.is_empty() {
                ui::info("No packs installed.");
            } else {
                ui::header("Installed packs");
                for (id, path) in &installed {
                    let sha256 = packs::dir_digest(path).unwrap_or_default();
                    ui::phase_ok(
                        id,
                        Some(&format!("{} · sha256:{}", path.display(), short(&sha256))),
                    );
                }
            }

            let installed_ids: Vec<&str> = installed.iter().map(|(id, _)| id.as_str()).collect();
            ui::header(&format!(
                "Built into oag v{} (used when not installed locally)",
                env!("CARGO_PKG_VERSION")
            ));
            for pack_id in packs::embedded_ids() {
                let marker = if installed_ids.contains(&pack_id) {
                    " (overridden by .oag/packs)"
                } else {
                    ""
                };
                let sha256 = packs::embedded_digest(pack_id).unwrap_or_default();
                ui::phase_ok(pack_id, Some(&format!("sha256:{}{marker}", short(&sha256))));
            }
            Ok(())
        }
        PacksAction::Install { source, id } => {
            if let Some(pack_id) = id {
                install_pack_by_id(&pack_id)?;
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
                let sha256 = packs::dir_digest(&target)?;
                packs::write_source_record(&target, &PackSource::Local, &sha256)?;
                ui::phase_ok(
                    "installed",
                    Some(&format!(
                        "{id} → {} · sha256:{}",
                        target.display(),
                        short(&sha256)
                    )),
                );
                Ok(())
            } else {
                anyhow::bail!("provide a source path or use --id <pack_id>");
            }
        }
        PacksAction::Remove { id } => {
            resolve::remove_pack(&id).map_err(|e| anyhow::anyhow!(e))?;
            ui::phase_ok("removed", Some(&id));
            Ok(())
        }
    }
}
