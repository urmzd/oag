#!/bin/sh
# install.sh — Installs the oag binary from GitHub releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/urmzd/oag/main/install.sh | sh
#
# The binary is verified against the release's SHA256SUMS before it is
# installed; a missing or mismatching checksum aborts the install.
#
# Environment variables:
#   OAG_VERSION     — version to install (e.g. "v0.9.0"); defaults to latest
#   OAG_INSTALL_DIR — installation directory; defaults to $HOME/.local/bin
#   OAG_SKIP_VERIFY — set to 1 to install without a checksum (only for
#                     releases published before SHA256SUMS existed, <= v0.20.6)

set -eu

REPO="urmzd/oag"

main() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  target="x86_64-unknown-linux-musl" ;;
                aarch64) target="aarch64-unknown-linux-musl" ;;
                *)       err "Unsupported Linux architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  target="x86_64-apple-darwin" ;;
                arm64)   target="aarch64-apple-darwin" ;;
                *)       err "Unsupported macOS architecture: $arch" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
            err "Windows is not supported by this installer. Download a binary from https://github.com/$REPO/releases/latest"
            ;;
        *)
            err "Unsupported operating system: $os"
            ;;
    esac

    if [ -n "${OAG_VERSION:-}" ]; then
        tag="$OAG_VERSION"
    else
        tag=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
            | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
        if [ -z "$tag" ]; then
            err "Failed to fetch latest release tag"
        fi
    fi

    artifact="oag-${target}"
    base_url="${OAG_DOWNLOAD_URL:-https://github.com/$REPO/releases/download}/${tag}"

    install_dir="${OAG_INSTALL_DIR:-$HOME/.local/bin}"
    mkdir -p "$install_dir"

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT INT TERM

    echo "Downloading oag $tag for $target..."
    curl -fsSL "$base_url/$artifact" -o "$tmp_dir/$artifact" \
        || err "Failed to download $base_url/$artifact"

    if [ "${OAG_SKIP_VERIFY:-0}" = "1" ]; then
        echo "Warning: OAG_SKIP_VERIFY=1, installing without checksum verification" >&2
    else
        verify "$base_url" "$artifact" "$tmp_dir"
    fi

    chmod +x "$tmp_dir/$artifact"
    mv "$tmp_dir/$artifact" "$install_dir/oag"

    echo "Installed oag to $install_dir/oag"

    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *) add_to_path "$install_dir" ;;
    esac
}

# verify BASE_URL ARTIFACT DIR — check DIR/ARTIFACT against the release's SHA256SUMS.
verify() {
    if ! curl -fsSL "$1/SHA256SUMS" -o "$3/SHA256SUMS"; then
        err "Could not download $1/SHA256SUMS to verify the binary. Releases up to v0.20.6 have no checksums; set OAG_SKIP_VERIFY=1 to install one of them unverified."
    fi

    expected=$(awk -v name="$2" '$2 == name || $2 == "*" name { print $1; exit }' "$3/SHA256SUMS")
    [ -n "$expected" ] || err "SHA256SUMS has no entry for $2"

    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$3/$2" | awk '{ print $1 }')
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$3/$2" | awk '{ print $1 }')
    else
        err "Neither sha256sum nor shasum is available to verify the download"
    fi

    if [ "$actual" != "$expected" ]; then
        err "Checksum mismatch for $2: expected $expected, got $actual. Not installing."
    fi
    echo "Verified SHA-256 checksum ($actual)"
}

add_to_path() {
    install_dir="$1"

    case "$(basename "$SHELL")" in
        zsh)  profile="$HOME/.zshrc" ;;
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                profile="$HOME/.bashrc"
            else
                profile="$HOME/.profile"
            fi
            ;;
        fish) profile="$HOME/.config/fish/config.fish" ;;
        *)    profile="$HOME/.profile" ;;
    esac

    if [ "$(basename "$SHELL")" = "fish" ]; then
        if ! grep -q "$install_dir" "$profile" 2>/dev/null; then
            mkdir -p "$(dirname "$profile")"
            echo "" >> "$profile"
            echo "# Added by oag installer" >> "$profile"
            echo "set -Ux fish_user_paths $install_dir \$fish_user_paths" >> "$profile"
            echo "Added $install_dir to $profile"
            echo "Restart your shell or run: source $profile"
        fi
    elif [ -n "$profile" ] && ! grep -q "$install_dir" "$profile" 2>/dev/null; then
        echo "" >> "$profile"
        echo "# Added by oag installer" >> "$profile"
        echo "export PATH=\"$install_dir:\$PATH\"" >> "$profile"
        echo "Added $install_dir to $profile"
        echo "Restart your shell or run: source $profile"
    fi
}

err() {
    echo "Error: $1" >&2
    exit 1
}

main
