use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const RAW_BASE: &str = "https://raw.githubusercontent.com/urmzd/oag";
const API_BASE: &str = "https://api.github.com/repos/urmzd/oag/contents/packs";

pub const KNOWN_PACKS: &[&str] = &["node-client", "react-swr-client", "fastapi-server"];

/// Parse a pack specifier like `node-client@v0.15.0` into `(pack_id, git_ref)`.
/// If no `@` is present, returns `(specifier, "main")`.
pub fn parse_pack_specifier(specifier: &str) -> (&str, &str) {
    match specifier.split_once('@') {
        Some((id, git_ref)) if !git_ref.is_empty() => (id, git_ref),
        _ => (specifier, "main"),
    }
}

/// Download a pack from GitHub into `target_dir` at the given git ref (tag, branch, or commit).
///
/// Fetches `oag.pack.toml` via raw URL, lists `templates/` via the GitHub Contents
/// API, then downloads each template file.
pub fn download_pack(pack_id: &str, git_ref: &str, target_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create {}", target_dir.display()))?;

    // 1. Download oag.pack.toml
    let pack_toml_url = format!("{RAW_BASE}/{git_ref}/packs/{pack_id}/oag.pack.toml");
    let pack_toml = fetch_text(&pack_toml_url).with_context(|| {
        format!("failed to download oag.pack.toml for '{pack_id}' at ref '{git_ref}'")
    })?;
    fs::write(target_dir.join("oag.pack.toml"), &pack_toml)?;

    // 2. List templates via GitHub Contents API
    let templates_api_url = format!("{API_BASE}/{pack_id}/templates?ref={git_ref}");
    let listing_json = fetch_text(&templates_api_url)
        .with_context(|| format!("failed to list templates for '{pack_id}' at ref '{git_ref}'"))?;

    let entries: Vec<GitHubContent> = serde_json::from_str(&listing_json)
        .with_context(|| "failed to parse GitHub API response")?;

    // 3. Download each template file
    let templates_dir = target_dir.join("templates");
    fs::create_dir_all(&templates_dir)?;

    for entry in &entries {
        if entry.r#type != "file" {
            continue;
        }
        // Use raw URL with the correct ref instead of the download_url (which may point to main)
        let url = format!(
            "{RAW_BASE}/{git_ref}/packs/{pack_id}/templates/{}",
            entry.name
        );
        let content =
            fetch_text(&url).with_context(|| format!("failed to download {}", entry.name))?;
        fs::write(templates_dir.join(&entry.name), &content)?;
    }

    Ok(())
}

fn fetch_text(url: &str) -> Result<String> {
    let mut response = ureq::get(url)
        .header("User-Agent", "oag-cli")
        .call()
        .with_context(|| format!("HTTP request failed: {url}"))?;

    let status = response.status();
    if status.as_u16() == 404 {
        anyhow::bail!("not found (404): {url}");
    }
    if status.as_u16() >= 400 {
        anyhow::bail!("HTTP {status}: {url}");
    }

    response
        .body_mut()
        .read_to_string()
        .with_context(|| format!("failed to read response body from {url}"))
}

#[derive(serde::Deserialize)]
struct GitHubContent {
    name: String,
    r#type: String,
    #[allow(dead_code)]
    download_url: Option<String>,
    #[allow(dead_code)]
    url: String,
}
