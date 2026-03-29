use std::path::{Path, PathBuf};

/// Get the platform-specific template pack directory.
pub fn templates_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("oag").join("templates"))
}

/// Resolve a pack directory by ID.
///
/// Resolution order:
/// 1. Explicit path (if provided in config)
/// 2. Installed packs in the platform data directory
/// 3. Returns None (caller falls back to embedded packs)
pub fn resolve_pack_path(pack_id: &str, explicit_path: Option<&Path>) -> Option<PathBuf> {
    // 1. Explicit path
    if let Some(path) = explicit_path
        && path.join("pack.toml").exists()
    {
        return Some(path.to_path_buf());
    }

    // 2. Installed packs in data directory
    if let Some(dir) = templates_dir() {
        let pack_dir = dir.join(pack_id);
        if pack_dir.join("pack.toml").exists() {
            return Some(pack_dir);
        }
    }

    // 3. Not found on disk
    None
}

/// List all installed template packs (id, path).
pub fn list_installed_packs() -> Vec<(String, PathBuf)> {
    let Some(dir) = templates_dir() else {
        return Vec::new();
    };
    if !dir.is_dir() {
        return Vec::new();
    }

    let mut packs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("pack.toml").exists() {
                let id = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                packs.push((id, path));
            }
        }
    }
    packs.sort_by(|a, b| a.0.cmp(&b.0));
    packs
}

/// Install a pack from a source directory to the templates directory.
pub fn install_pack(source: &Path, pack_id: &str) -> Result<PathBuf, String> {
    let Some(dir) = templates_dir() else {
        return Err("could not determine data directory".to_string());
    };
    let target = dir.join(pack_id);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("failed to remove existing pack: {e}"))?;
    }
    copy_dir_recursive(source, &target)?;
    Ok(target)
}

/// Remove an installed pack.
pub fn remove_pack(pack_id: &str) -> Result<(), String> {
    let Some(dir) = templates_dir() else {
        return Err("could not determine data directory".to_string());
    };
    let target = dir.join(pack_id);
    if !target.exists() {
        return Err(format!("pack '{pack_id}' is not installed"));
    }
    std::fs::remove_dir_all(&target)
        .map_err(|e| format!("failed to remove pack '{pack_id}': {e}"))?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {e}", dst.display()))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| format!("failed to read {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "failed to copy {} to {}: {e}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}
