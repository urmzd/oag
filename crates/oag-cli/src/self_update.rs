use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

/// Result of a self-update check.
#[derive(Debug)]
pub enum UpdateResult {
    /// Already on the latest version.
    AlreadyUpToDate,
    /// Updated from old to new version.
    Updated { from: String, to: String },
}

/// Detect the current platform target triple.
fn detect_target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        (os, arch) => bail!("unsupported platform: {os}/{arch}"),
    }
}

fn gh_get(url: &str) -> Result<ureq::Body> {
    let mut req = ureq::get(url).header("Accept", "application/vnd.github+json");

    if let Ok(token) = std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN")) {
        req = req.header("Authorization", format!("token {token}"));
    }

    let resp = req.call().context("HTTP request failed")?;
    Ok(resp.into_body())
}

fn fetch_latest_release(repo: &str) -> Result<Release> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let mut body = gh_get(&url)?;
    let release: Release = body.read_json().context("failed to parse release JSON")?;
    Ok(release)
}

fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let body = gh_get(url)?;
    let mut bytes = Vec::new();
    body.into_reader()
        .read_to_end(&mut bytes)
        .context("failed to read response body")?;
    Ok(bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Atomically replace a binary file with new content.
fn atomic_replace(target: &PathBuf, new_bytes: &[u8]) -> Result<()> {
    let backup = target.with_extension("old");
    let tmp = target.with_extension("new");

    fs::write(&tmp, new_bytes).context("failed to write new binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))
            .context("failed to set executable permissions")?;
    }

    if target.exists() {
        fs::rename(target, &backup).context("failed to backup current binary")?;
    }
    if let Err(e) = fs::rename(&tmp, target) {
        if backup.exists() {
            let _ = fs::rename(&backup, target);
        }
        return Err(e).context("failed to replace binary");
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}

/// Release asset name for a binary and target: `{binary}-{target}`, with an
/// `.exe` suffix on Windows (as the release workflow uploads it).
fn asset_name_for(binary_name: &str, target: &str) -> String {
    if target.contains("windows") {
        format!("{binary_name}-{target}.exe")
    } else {
        format!("{binary_name}-{target}")
    }
}

/// The expected SHA-256 of `asset_name`, from its `.sha256` sidecar or the
/// release's `SHA256SUMS`. Fails closed: a release without either, or a
/// checksum file that cannot be downloaded or has no entry, is an error.
fn expected_sha256(
    assets: &[ReleaseAsset],
    asset_name: &str,
    fetch: impl Fn(&str) -> Result<Vec<u8>>,
) -> Result<String> {
    let find = |name: &str| assets.iter().find(|a| a.name == name);

    if let Some(sidecar) = find(&format!("{asset_name}.sha256")) {
        let text = String::from_utf8(fetch(&sidecar.browser_download_url)?)
            .context("checksum sidecar is not UTF-8")?;
        return parse_hex(text.split_whitespace().next().unwrap_or(""))
            .with_context(|| format!("malformed checksum in {}", sidecar.name));
    }

    if let Some(sums) = find("SHA256SUMS") {
        let text = String::from_utf8(fetch(&sums.browser_download_url)?)
            .context("SHA256SUMS is not UTF-8")?;
        let entry = text.lines().find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            (name == asset_name).then(|| hash.to_string())
        });
        return match entry {
            Some(hash) => parse_hex(&hash).context("malformed checksum in SHA256SUMS"),
            None => bail!("SHA256SUMS has no entry for {asset_name}"),
        };
    }

    bail!(
        "the release publishes no checksum for {asset_name} (no {asset_name}.sha256 or \
         SHA256SUMS); refusing to install an unverified binary"
    )
}

fn parse_hex(hash: &str) -> Result<String> {
    if hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(hash.to_ascii_lowercase())
    } else {
        bail!("not a SHA-256 hex digest: {hash:?}")
    }
}

/// Self-update the current binary from GitHub Releases.
///
/// `repo` — "owner/name" format (e.g., "urmzd/sr").
/// `current_version` — current version string (without "v" prefix).
/// `binary_name` — name prefix in release assets (e.g., "sr").
///
/// Assets are expected to be named `{binary_name}-{target}` (plus `.exe` on
/// Windows). The download is verified against the release's checksums and
/// rejected when none are published.
pub fn self_update(repo: &str, current_version: &str, binary_name: &str) -> Result<UpdateResult> {
    let release = fetch_latest_release(repo)?;
    let latest_version = release.tag_name.trim_start_matches('v');

    if latest_version == current_version {
        return Ok(UpdateResult::AlreadyUpToDate);
    }

    let target = detect_target()?;
    let asset_name = asset_name_for(binary_name, target);

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow::anyhow!("no asset found for {asset_name}"))?;

    let expected = expected_sha256(&release.assets, &asset_name, download_bytes)?;

    eprintln!("downloading {binary_name} {latest_version} for {target}...");

    let bytes = download_bytes(&asset.browser_download_url)?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = hex_encode(&hasher.finalize());
    if actual != expected {
        bail!("SHA256 mismatch: expected {expected}, got {actual}");
    }

    let current_exe = std::env::current_exe().context("cannot determine current executable")?;
    atomic_replace(&current_exe, &bytes)?;

    Ok(UpdateResult::Updated {
        from: current_version.to_string(),
        to: latest_version.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "cddc7620a29f8dd47032e99e54f10247e151498288cc53e984599f07c30b6e3b";

    fn asset(name: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.test/{name}"),
        }
    }

    fn serve(files: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Result<Vec<u8>> {
        move |url: &str| {
            files
                .iter()
                .find(|(name, _)| url.ends_with(&format!("/{name}")))
                .map(|(_, body)| body.as_bytes().to_vec())
                .ok_or_else(|| anyhow::anyhow!("404 {url}"))
        }
    }

    #[test]
    fn windows_asset_has_exe_suffix() {
        assert_eq!(
            asset_name_for("oag", "x86_64-pc-windows-msvc"),
            "oag-x86_64-pc-windows-msvc.exe"
        );
        assert_eq!(
            asset_name_for("oag", "aarch64-apple-darwin"),
            "oag-aarch64-apple-darwin"
        );
    }

    #[test]
    fn checksum_comes_from_the_sidecar_or_sha256sums() {
        let name = "oag-aarch64-apple-darwin";
        let assets = [asset(name), asset(&format!("{name}.sha256"))];
        let sidecar: &'static [(&str, &str)] = &[(
            "oag-aarch64-apple-darwin.sha256",
            "cddc7620a29f8dd47032e99e54f10247e151498288cc53e984599f07c30b6e3b  oag-aarch64-apple-darwin\n",
        )];
        assert_eq!(
            expected_sha256(&assets, name, serve(sidecar)).unwrap(),
            HASH
        );

        let assets = [asset(name), asset("SHA256SUMS")];
        let sums: &'static [(&str, &str)] = &[(
            "SHA256SUMS",
            "0000000000000000000000000000000000000000000000000000000000000000  oag-x86_64-apple-darwin\ncddc7620a29f8dd47032e99e54f10247e151498288cc53e984599f07c30b6e3b  oag-aarch64-apple-darwin\n",
        )];
        assert_eq!(expected_sha256(&assets, name, serve(sums)).unwrap(), HASH);
    }

    #[test]
    fn missing_or_unreadable_checksums_fail_closed() {
        let name = "oag-aarch64-apple-darwin";
        // No checksum published at all (releases up to v0.20.6).
        assert!(expected_sha256(&[asset(name)], name, serve(&[])).is_err());
        // The sidecar exists but cannot be downloaded.
        let assets = [asset(name), asset(&format!("{name}.sha256"))];
        assert!(expected_sha256(&assets, name, serve(&[])).is_err());
        // SHA256SUMS without an entry for this asset.
        let assets = [asset(name), asset("SHA256SUMS")];
        let other: &'static [(&str, &str)] = &[(
            "SHA256SUMS",
            "cddc7620a29f8dd47032e99e54f10247e151498288cc53e984599f07c30b6e3b  oag-x86_64-apple-darwin\n",
        )];
        assert!(expected_sha256(&assets, name, serve(other)).is_err());
        // A malformed digest.
        let assets = [asset(name), asset(&format!("{name}.sha256"))];
        let bad: &'static [(&str, &str)] = &[("oag-aarch64-apple-darwin.sha256", "nothex  x\n")];
        assert!(expected_sha256(&assets, name, serve(bad)).is_err());
    }
}
