//! Built-in packs embedded in the binary, pack digests, and install records.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use oag_core::engine::pack::TemplatePack;
use sha2::{Digest, Sha256};

include!(concat!(env!("OUT_DIR"), "/embedded_packs.rs"));

/// File name of the record written next to an installed pack.
pub const SOURCE_FILE: &str = "oag.source.toml";

/// Where a resolved pack came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSource {
    /// Compiled into this oag binary.
    Embedded,
    /// A directory under `.oag/packs/` (installed or committed).
    Local,
    /// Downloaded from GitHub at an explicit ref.
    GitHub(String),
}

impl std::fmt::Display for PackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackSource::Embedded => write!(f, "built into oag v{}", env!("CARGO_PKG_VERSION")),
            PackSource::Local => write!(f, ".oag/packs"),
            PackSource::GitHub(git_ref) => write!(f, "github@{git_ref}"),
        }
    }
}

/// IDs of the packs embedded in this binary.
pub fn embedded_ids() -> Vec<&'static str> {
    EMBEDDED_PACKS.iter().map(|(id, _)| *id).collect()
}

fn embedded_files(id: &str) -> Option<&'static [(&'static str, &'static str)]> {
    EMBEDDED_PACKS
        .iter()
        .find(|(pack_id, _)| *pack_id == id)
        .map(|(_, files)| *files)
}

/// Load an embedded pack (without resolving `extends`).
pub fn load_embedded(id: &str) -> Option<Result<TemplatePack>> {
    let files = embedded_files(id)?;
    let manifest = files
        .iter()
        .find(|(path, _)| *path == "oag.pack.toml")
        .map(|(_, content)| *content)?;
    let templates = files
        .iter()
        .filter_map(|(path, content)| path.strip_prefix("templates/").map(|name| (name, *content)));
    Some(
        TemplatePack::from_files(manifest, templates)
            .map_err(|e| anyhow::anyhow!("failed to load built-in pack '{id}': {e}")),
    )
}

/// SHA-256 digest of an embedded pack's files.
pub fn embedded_digest(id: &str) -> Option<String> {
    embedded_files(id).map(|files| {
        digest(
            files
                .iter()
                .map(|(path, content)| (path.to_string(), content.as_bytes().to_vec())),
        )
    })
}

/// Write an embedded pack to `target` (replacing it).
pub fn write_embedded(id: &str, target: &Path) -> Result<bool> {
    let Some(files) = embedded_files(id) else {
        return Ok(false);
    };
    if target.exists() {
        fs::remove_dir_all(target)
            .with_context(|| format!("failed to remove {}", target.display()))?;
    }
    for (rel, content) in files {
        let path = target.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(true)
}

/// SHA-256 digest of a pack directory: `oag.pack.toml` plus `templates/*`.
/// Matches [`embedded_digest`] for an unmodified copy of an embedded pack.
pub fn dir_digest(dir: &Path) -> Result<String> {
    let mut files = vec![(
        "oag.pack.toml".to_string(),
        fs::read(dir.join("oag.pack.toml"))?,
    )];
    let templates = dir.join("templates");
    if templates.is_dir() {
        for entry in fs::read_dir(&templates)? {
            let path = entry?.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                files.push((format!("templates/{name}"), fs::read(&path)?));
            }
        }
    }
    Ok(digest(files.into_iter()))
}

/// Order-independent digest over `(relative path, bytes)` pairs.
fn digest(files: impl Iterator<Item = (String, Vec<u8>)>) -> String {
    let mut files: Vec<(String, Vec<u8>)> = files.collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (path, content) in files {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Record how a pack was installed, next to it, so `oag packs list` and CI can
/// tell what was resolved.
pub fn write_source_record(target: &Path, source: &PackSource, sha256: &str) -> Result<()> {
    let (kind, git_ref) = match source {
        PackSource::Embedded => ("embedded", None),
        PackSource::Local => ("path", None),
        PackSource::GitHub(r) => ("github", Some(r.as_str())),
    };
    let mut record = format!(
        "# Written by oag when this pack was installed. Not read during generation.\n\
         source = \"{kind}\"\n\
         oag_version = \"{}\"\n\
         sha256 = \"{sha256}\"\n",
        env!("CARGO_PKG_VERSION")
    );
    if let Some(r) = git_ref {
        record.push_str(&format!("ref = \"{r}\"\n"));
    }
    fs::write(target.join(SOURCE_FILE), record)
        .with_context(|| format!("failed to write {}", target.join(SOURCE_FILE).display()))
}

/// The install record written next to a pack (`oag.source.toml`).
#[derive(Debug, Default, serde::Deserialize)]
pub struct SourceRecord {
    pub source: Option<String>,
    pub oag_version: Option<String>,
    pub sha256: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
}

/// Read a pack's install record, if it has one.
pub fn read_source_record(dir: &Path) -> Option<SourceRecord> {
    let text = fs::read_to_string(dir.join(SOURCE_FILE)).ok()?;
    toml::from_str(&text).ok()
}

/// Why a local pack that shadows a built-in pack is stale, if it is.
///
/// A pack in `.oag/packs/` takes precedence over the built-in pack of the
/// same id. Templates from another oag version can render code that does not
/// compile against this engine (or silently keep old runtime behaviour), and
/// oag v0.20 and earlier downloaded every missing pack into `.oag/packs/`, so
/// most upgraded projects carry such a copy. A local copy of a built-in pack
/// is therefore only used when it was installed by this oag version.
pub fn stale_reason(id: &str, dir: &Path) -> Option<String> {
    if !embedded_ids().contains(&id) {
        return None;
    }
    let current = env!("CARGO_PKG_VERSION");
    match read_source_record(dir).and_then(|r| r.oag_version) {
        None => Some(
            "has no install record (it was downloaded by oag v0.20 or earlier, or copied by hand)"
                .to_string(),
        ),
        Some(v) if v != current => Some(format!("was installed by oag v{v}")),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_packs_are_embedded() {
        let ids = embedded_ids();
        for id in ["node-client", "react-swr-client", "fastapi-server"] {
            assert!(ids.contains(&id), "{id} should be embedded, got {ids:?}");
            let pack = load_embedded(id).expect("embedded").expect("loads");
            assert_eq!(pack.manifest.pack.id, id);
            assert!(!pack.templates.is_empty(), "{id} has templates");
        }
    }

    #[test]
    fn embedded_packs_match_the_working_tree() {
        // The embedded copy is the repository's `packs/` at build time, so the
        // digest of the source directory equals the embedded digest.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
        for id in embedded_ids() {
            assert_eq!(
                dir_digest(&root.join(id)).unwrap(),
                embedded_digest(id).unwrap(),
                "{id}"
            );
        }
    }

    #[test]
    fn local_copy_of_a_built_in_pack_without_a_current_record_is_stale() {
        let dir = std::env::temp_dir().join(format!("oag-stale-{}", std::process::id()));
        assert!(write_embedded("node-client", &dir).unwrap());
        assert!(
            stale_reason("node-client", &dir)
                .unwrap()
                .contains("no install record")
        );

        fs::write(
            dir.join(SOURCE_FILE),
            "source = \"github\"\noag_version = \"0.0.1\"\n",
        )
        .unwrap();
        assert!(
            stale_reason("node-client", &dir)
                .unwrap()
                .contains("oag v0.0.1")
        );

        let sha = dir_digest(&dir).unwrap();
        write_source_record(&dir, &PackSource::Embedded, &sha).unwrap();
        assert_eq!(stale_reason("node-client", &dir), None);

        // Custom packs (ids that are not built in) are never stale.
        assert_eq!(stale_reason("my-pack", &dir), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn written_embedded_pack_round_trips() {
        let dir = std::env::temp_dir().join(format!("oag-embedded-{}", std::process::id()));
        assert!(write_embedded("node-client", &dir).unwrap());
        assert_eq!(
            dir_digest(&dir).unwrap(),
            embedded_digest("node-client").unwrap()
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
