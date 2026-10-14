//! Embed the built-in template packs into the `oag` binary.
//!
//! Generation must be hermetic: the packs that ship with a given oag version
//! are compiled in, so `oag generate` never fetches templates from the network
//! (previously a missing pack was downloaded from the repository's `main`
//! branch, which made output change over time for the same oag version).
//!
//! Packs are read from `crates/oag-cli/packs`, a symlink to the workspace's
//! `packs/` directory. `cargo package` follows the symlink, so the published
//! crate carries a real copy and `cargo install oag` embeds them too. Where the
//! symlink is not materialised (a Windows checkout without symlink support),
//! the workspace path is used directly.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let candidates = [manifest_dir.join("packs"), manifest_dir.join("../../packs")];
    let root = candidates.iter().find(|p| p.is_dir()).cloned();

    let mut out = String::from(
        "/// Built-in packs embedded at compile time: `(pack id, [(relative path, content)])`.\n\
         pub static EMBEDDED_PACKS: &[(&str, &[(&str, &str)])] = &[\n",
    );

    match &root {
        Some(root) => {
            println!("cargo:rerun-if-changed={}", root.display());
            for (id, dir) in pack_dirs(root) {
                println!("cargo:rerun-if-changed={}", dir.display());
                writeln!(out, "    ({id:?}, &[").unwrap();
                for (rel, path) in pack_files(&dir) {
                    println!("cargo:rerun-if-changed={}", path.display());
                    let abs = path.canonicalize().unwrap_or(path);
                    writeln!(
                        out,
                        "        ({rel:?}, include_str!({:?})),",
                        abs.display().to_string()
                    )
                    .unwrap();
                }
                writeln!(out, "    ]),").unwrap();
            }
        }
        None => panic!(
            "no packs directory found (looked for {} and {}); oag cannot be built without its \
             built-in packs",
            candidates[0].display(),
            candidates[1].display()
        ),
    }
    out.push_str("];\n");

    let dest = PathBuf::from(env::var("OUT_DIR").unwrap()).join("embedded_packs.rs");
    fs::write(dest, out).unwrap();
}

/// Pack directories (containing `oag.pack.toml`), sorted by id.
fn pack_dirs(root: &Path) -> Vec<(String, PathBuf)> {
    let mut dirs: Vec<(String, PathBuf)> = fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("oag.pack.toml").is_file())
        .map(|p| (p.file_name().unwrap().to_string_lossy().to_string(), p))
        .collect();
    dirs.sort();
    dirs
}

/// `oag.pack.toml` plus every file under `templates/`, as `(relative path, path)`.
fn pack_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut files = vec![("oag.pack.toml".to_string(), dir.join("oag.pack.toml"))];
    let templates = dir.join("templates");
    if templates.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&templates)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        entries.sort();
        for path in entries {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            files.push((format!("templates/{name}"), path));
        }
    }
    files
}
